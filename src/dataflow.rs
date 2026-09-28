use index_vec::{IndexVec, define_index_type};
use pliron::{
    basic_block::BasicBlock,
    builtin::op_interfaces::BranchOpInterface,
    context::{Context, Ptr},
    linked_list::{ContainsLinkedList, LinkedList},
    op::op_cast,
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    region::Region,
    result::Error as PlironError,
    value::{DefiningEntity, Value},
};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

// === DataflowGraph === //

define_index_type! {
    struct DataflowNodeIdx = u32;
}

define_index_type! {
    struct DataflowEffectIdx = u32;
}

define_index_type! {
    struct DataflowStateIdx = u32;
}

pub struct DataflowGraph {
    node_defs: IndexVec<DataflowNodeIdx, DataflowNode>,
    op_map: FxHashMap<Ptr<Operation>, DataflowNodeIdx>,
    bb_map: FxHashMap<Ptr<BasicBlock>, SmallVec<[DataflowNodeIdx; 2]>>,
    effect_slots: IndexVec<DataflowEffectIdx, DataflowNodeIdx>,
    state_slots: IndexVec<DataflowStateIdx, SmallVec<[DataflowNodeIdx; 1]>>,
}

enum DataflowNode {
    Phi {
        input_states: SmallVec<[DataflowStateIdx; 2]>,
        output_state: DataflowStateIdx,
    },
    Stmt {
        operation: Ptr<Operation>,
        input_effect: DataflowEffectIdx,
        output_effect: DataflowEffectIdx,
        input_states: SmallVec<[DataflowStateIdx; 2]>,
        output_state: Option<DataflowStateIdx>,
    },
    Terminator {
        operation: Ptr<Operation>,
        input_effect: DataflowEffectIdx,
        input_states: SmallVec<[DataflowStateIdx; 2]>,
        output_effects: SmallVec<[DataflowEffectIdx; 2]>,
    },
}

impl DataflowGraph {
    pub fn new(ctx: &Context, region: Ptr<Region>) -> Self {
        let mut graph = DataflowGraph {
            node_defs: IndexVec::default(),
            op_map: FxHashMap::default(),
            bb_map: FxHashMap::default(),
            effect_slots: IndexVec::default(),
            state_slots: IndexVec::default(),
        };

        // Create placeholder phi nodes, statements, and terminators.
        for basic_block in region.deref(ctx).iter(ctx) {
            for operation in basic_block.deref(ctx).iter(ctx) {
                let operation_r = operation.deref(ctx);

                let input_effect = graph.effect_slots.push(graph.node_defs.next_idx());

                if operation_r.get_next().is_some() {
                    let output_state = match operation_r.get_num_results() {
                        0 => None,
                        1 => Some(graph.state_slots.push(SmallVec::from_iter([]))),
                        _ => unreachable!(),
                    };

                    let node_idx = graph.node_defs.push(DataflowNode::Stmt {
                        operation,
                        input_effect: input_effect,
                        output_effect: DataflowEffectIdx::from_usize(DataflowEffectIdx::MAX_INDEX),
                        input_states: SmallVec::new(),
                        output_state,
                    });

                    graph.op_map.insert(operation, node_idx);
                } else {
                    let node_idx = graph.node_defs.push(DataflowNode::Terminator {
                        operation,
                        input_effect,
                        input_states: SmallVec::new(),
                        output_effects: SmallVec::new(),
                    });

                    graph.op_map.insert(operation, node_idx);
                }
            }

            let argument_phi_nodes = (0..basic_block.deref(ctx).get_num_arguments())
                .map(|_| {
                    let output_state = graph.state_slots.push(SmallVec::new());

                    graph.node_defs.push(DataflowNode::Phi {
                        input_states: SmallVec::new(),
                        output_state,
                    })
                })
                .collect();

            graph.bb_map.insert(basic_block, argument_phi_nodes);
        }

        // Connect up everything.
        fn lookup_state(graph: &DataflowGraph, ctx: &Context, value: Value) -> DataflowStateIdx {
            match value.defining_entity() {
                DefiningEntity::Op(op) => match graph.node_defs[graph.op_map[&op]] {
                    DataflowNode::Phi { .. }
                    | DataflowNode::Terminator { .. }
                    | DataflowNode::Stmt {
                        output_state: None, ..
                    } => unreachable!(),
                    DataflowNode::Stmt {
                        output_state: Some(output_state),
                        ..
                    } => output_state,
                },
                DefiningEntity::Block(bb) => {
                    let phi_node = graph.bb_map[&bb][value.find_index(ctx)];

                    let DataflowNode::Phi { output_state, .. } = graph.node_defs[phi_node] else {
                        unreachable!()
                    };

                    output_state
                }
            }
        }

        for node_idx in graph.node_defs.indices() {
            match &graph.node_defs[node_idx] {
                DataflowNode::Phi { .. } => {
                    // (connected in terminators)
                }
                DataflowNode::Stmt {
                    operation,
                    input_effect: _, // (already init)
                    output_effect: _,
                    input_states: _,
                    output_state: _, // (already init)
                } => {
                    let operation_r = operation.deref(ctx);
                    let operation_succ = operation_r.get_next().unwrap();

                    // Determine `output_effect`
                    let (DataflowNode::Stmt {
                        input_effect: init_output_effect,
                        ..
                    }
                    | DataflowNode::Terminator {
                        input_effect: init_output_effect,
                        ..
                    }) = graph.node_defs[graph.op_map[&operation_succ]]
                    else {
                        unreachable!()
                    };

                    // Determine `input_states`
                    let init_input_states = operation_r
                        .operands()
                        .map(|value| lookup_state(&graph, ctx, value))
                        .collect::<SmallVec<[DataflowStateIdx; 2]>>();

                    for &input_state in &init_input_states {
                        graph.state_slots[input_state].push(node_idx);
                    }

                    // Write out states
                    let DataflowNode::Stmt {
                        output_effect,
                        input_states,
                        ..
                    } = &mut graph.node_defs[node_idx]
                    else {
                        unreachable!()
                    };

                    *output_effect = init_output_effect;
                    *input_states = init_input_states;
                }
                DataflowNode::Terminator {
                    operation,
                    input_effect: _, // (already init)
                    input_states: _,
                    output_effects: _,
                } => {
                    let operation = *operation;
                    let operation_r = operation.deref(ctx);

                    // Determine `input_states`
                    // TODO: possibly truncate states that are only used in forwarding
                    let init_input_states = operation_r
                        .operands()
                        .map(|value| lookup_state(&graph, ctx, value))
                        .collect::<SmallVec<[DataflowStateIdx; 2]>>();

                    for &input_state in &init_input_states {
                        graph.state_slots[input_state].push(node_idx);
                    }

                    // Determine `output_effects`
                    let init_output_effects = operation_r
                        .successors()
                        .into_iter()
                        .map(|bb| {
                            let first_op = bb.deref(ctx).get_head().unwrap();

                            let (DataflowNode::Stmt { input_effect, .. }
                            | DataflowNode::Terminator { input_effect, .. }) =
                                graph.node_defs[graph.op_map[&first_op]]
                            else {
                                unreachable!()
                            };

                            input_effect
                        })
                        .collect::<SmallVec<[_; 2]>>();

                    // Write out states
                    let DataflowNode::Terminator {
                        input_states,
                        output_effects,
                        ..
                    } = &mut graph.node_defs[node_idx]
                    else {
                        unreachable!()
                    };

                    *input_states = init_input_states;
                    *output_effects = init_output_effects;

                    // Link up phi node inputs.
                    if operation_r.get_num_successors() == 0 {
                        // Return and unreachable aren't considered branches.
                        continue;
                    }

                    let operation_b = Operation::get_op_dyn(operation, ctx);
                    let operation_b = op_cast::<dyn BranchOpInterface>(&*operation_b).unwrap();

                    for succ_idx in 0..operation_r.get_num_successors() {
                        let succ = operation_r.get_successor(succ_idx);
                        let phi_nodes = graph.bb_map[&succ].clone();

                        for (src_value, dst_node) in operation_b
                            .successor_operands(ctx, succ_idx)
                            .into_iter()
                            .zip(phi_nodes)
                        {
                            let src_state = lookup_state(&graph, ctx, src_value);

                            let DataflowNode::Phi {
                                input_states,
                                output_state: _,
                            } = &mut graph.node_defs[dst_node]
                            else {
                                unreachable!();
                            };

                            input_states.push(src_state);
                            graph.state_slots[src_state].push(dst_node);
                        }
                    }
                }
            }
        }

        graph
    }
}

impl Analysis for DataflowGraph {
    fn name(&self) -> &str {
        "dataflow graph"
    }

    fn compute(
        op: Ptr<Operation>,
        ctx: &Context,
        _analyses: &mut AnalysisManager,
    ) -> Result<Self, PlironError>
    where
        Self: Sized,
    {
        Ok(DataflowGraph::new(ctx, op.deref(ctx).get_region(0)))
    }
}
