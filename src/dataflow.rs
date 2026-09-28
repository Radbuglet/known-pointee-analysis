use index_vec::{IndexVec, define_index_type};
use pliron::{
    basic_block::BasicBlock,
    context::{Context, Ptr},
    linked_list::{ContainsLinkedList, LinkedList},
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
                DefiningEntity::Block(bb) => todo!(),
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

                    let init_input_states = operation_r
                        .operands()
                        .map(|value| lookup_state(&graph, ctx, value))
                        .collect::<SmallVec<[DataflowStateIdx; 2]>>();

                    for &input_state in &init_input_states {
                        graph.state_slots[input_state].push(node_idx);
                    }

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
                    let operation_r = operation.deref(ctx);

                    // TODO
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
