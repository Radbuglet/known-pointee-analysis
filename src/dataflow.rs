use index_vec::{IndexVec, define_index_type};
use pliron::{
    basic_block::BasicBlock,
    builtin::op_interfaces::{BranchOpInterface, OperandSegmentInterface},
    context::{Context, Ptr},
    linked_list::{ContainsLinkedList, LinkedList},
    op::op_cast,
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    region::Region,
    result::Error as PlironError,
    value::DefiningEntity,
};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

// === DataflowGraph === //

define_index_type! {
    struct DataflowNodeIdx = u32;
}

pub struct DataflowGraph {
    nodes: IndexVec<DataflowNodeIdx, DataflowNode>,
    bb_mapping: FxHashMap<Ptr<BasicBlock>, DataflowNodeIdx>,
    op_mapping: FxHashMap<Ptr<Operation>, DataflowNodeIdx>,
}

enum DataflowNode {
    /// A start-of-block node which stashes incoming effects and block variable states. Can also be
    /// an input block.
    StartOfBlock {
        block: Ptr<BasicBlock>,

        /// The statement to which this effect is forward.
        effect_successor: DataflowNodeIdx,

        /// Nodes which consume this block's variable states.
        output_consumers: SmallVec<[DataflowOutput; 1]>,
    },
    /// A statement node which may produce a result and propagate that result elsewhere.
    Stmt {
        operation: Ptr<Operation>,

        /// `Stmt` and `StartOfBlock` nodes supplying our operand variable states.
        operands: SmallVec<[DataflowOperand; 2]>,

        /// `Stmt` and `StartOfBlock` nodes which consume our result.
        output_consumers: SmallVec<[DataflowOutput; 1]>,

        /// Subsequent node in this basic block.
        effect_successor: DataflowNodeIdx,
    },
    /// A terminator node which may propagate its effect to multiple different targets.
    Terminator {
        operation: Ptr<Operation>,

        /// `Stmt` and `StartOfBlock` nodes supplying our operand variable states.
        /// Does not count forwarded operands.
        direct_operands: SmallVec<[DataflowOperand; 2]>,

        /// Where each successor `StartOfBlock` lives.
        effect_successors: SmallVec<[DataflowNodeIdx; 2]>,
    },
}

struct DataflowOperand {
    node: DataflowNodeIdx,
    output_idx: u32,
}

struct DataflowOutput {
    node: DataflowNodeIdx,
    output_idx_if_start: u32,
}

impl DataflowGraph {
    pub fn new(ctx: &Context, region: Ptr<Region>) -> Self {
        let mut graph = Self {
            nodes: IndexVec::new(),
            bb_mapping: FxHashMap::default(),
            op_mapping: FxHashMap::default(),
        };

        // Create placeholder nodes
        for bb in region.deref(ctx).iter(ctx) {
            graph.bb_mapping.insert(
                bb,
                graph.nodes.push(DataflowNode::StartOfBlock {
                    block: bb,
                    effect_successor: DataflowNodeIdx::new(DataflowNodeIdx::MAX_INDEX),
                    output_consumers: SmallVec::new(),
                }),
            );

            for op in bb.deref(ctx).iter(ctx) {
                if op.deref(ctx).get_next().is_none() {
                    graph.op_mapping.insert(
                        op,
                        graph.nodes.push(DataflowNode::Terminator {
                            operation: op,
                            direct_operands: SmallVec::new(),
                            effect_successors: SmallVec::new(),
                        }),
                    );
                } else {
                    graph.op_mapping.insert(
                        op,
                        graph.nodes.push(DataflowNode::Stmt {
                            operation: op,
                            operands: SmallVec::new(),
                            output_consumers: SmallVec::new(),
                            effect_successor: DataflowNodeIdx::new(DataflowNodeIdx::MAX_INDEX),
                        }),
                    );
                }
            }
        }

        // Link up forward references (i.e. effects and forwarded arguments)
        for node_idx in graph.nodes.indices() {
            match &mut graph.nodes[node_idx] {
                DataflowNode::StartOfBlock {
                    block,
                    effect_successor,
                    output_consumers: _,
                } => {
                    *effect_successor = graph.op_mapping[&block.deref(ctx).get_head().unwrap()];
                }
                DataflowNode::Stmt {
                    operation,
                    operands: _,
                    output_consumers: _,
                    effect_successor,
                } => {
                    *effect_successor = graph.op_mapping[&operation.deref(ctx).get_next().unwrap()];
                }
                DataflowNode::Terminator {
                    operation,
                    direct_operands: _,
                    effect_successors,
                } => {
                    *effect_successors = operation
                        .deref(ctx)
                        .successors()
                        .map(|bb| graph.bb_mapping[&bb])
                        .collect();

                    if operation.deref(ctx).get_num_successors() == 0 {
                        // For return and friends, which are not considered branches.
                        continue;
                    }

                    let dyn_operation = Operation::get_op_dyn(*operation, ctx);
                    let dyn_operation = op_cast::<dyn BranchOpInterface>(&*dyn_operation).unwrap();

                    for succ_idx in 0..operation.deref(ctx).get_num_successors() {
                        for (output_idx, operand) in dyn_operation
                            .successor_operands(ctx, succ_idx)
                            .into_iter()
                            .enumerate()
                        {
                            match operand.defining_entity() {
                                DefiningEntity::Op(op) => {
                                    let op_idx = graph.op_mapping[&op];

                                    let DataflowNode::Stmt {
                                        output_consumers: op_output_consumers,
                                        ..
                                    } = &mut graph.nodes[op_idx]
                                    else {
                                        unreachable!()
                                    };

                                    op_output_consumers.push(DataflowOutput {
                                        node: node_idx,
                                        output_idx_if_start: output_idx as u32,
                                    });
                                }
                                DefiningEntity::Block(bb) => {
                                    let bb_idx = graph.bb_mapping[&bb];

                                    let DataflowNode::StartOfBlock {
                                        output_consumers: bb_output_consumers,
                                        ..
                                    } = &mut graph.nodes[bb_idx]
                                    else {
                                        unreachable!()
                                    };

                                    bb_output_consumers.push(DataflowOutput {
                                        node: node_idx,
                                        output_idx_if_start: output_idx as u32,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Link up backwards references (i.e. direct operands)
        for node_idx in graph.nodes.indices() {
            let operands = match graph.nodes[node_idx] {
                DataflowNode::StartOfBlock { .. } => {
                    // (no backward operands)
                    continue;
                }
                DataflowNode::Stmt { operation, .. } => {
                    operation.deref(ctx).operands().collect::<Vec<_>>()
                }
                DataflowNode::Terminator { operation, .. } => {
                    let dyn_operation = Operation::get_op_dyn(operation, ctx);

                    // TODO: there has to be a better way to find these :(
                    if let Some(segments) = op_cast::<dyn OperandSegmentInterface>(&*dyn_operation)
                    {
                        segments.get_segment(ctx, 0)
                    } else {
                        operation.deref(ctx).operands().collect::<Vec<_>>()
                    }
                }
            };

            let node_operands_init = operands
                .into_iter()
                .map(|operand| match operand.defining_entity() {
                    DefiningEntity::Op(op) => {
                        let op_idx = graph.op_mapping[&op];

                        let DataflowNode::Stmt {
                            output_consumers: op_output_consumers,
                            ..
                        } = &mut graph.nodes[op_idx]
                        else {
                            unreachable!()
                        };

                        op_output_consumers.push(DataflowOutput {
                            node: node_idx,
                            // We're not a start node.
                            output_idx_if_start: 0,
                        });

                        DataflowOperand {
                            node: op_idx,
                            output_idx: 0,
                        }
                    }
                    DefiningEntity::Block(bb) => {
                        let bb_idx = graph.bb_mapping[&bb];

                        let DataflowNode::StartOfBlock {
                            output_consumers: bb_output_consumers,
                            ..
                        } = &mut graph.nodes[bb_idx]
                        else {
                            unreachable!()
                        };

                        bb_output_consumers.push(DataflowOutput {
                            node: node_idx,
                            // We're not a start node.
                            output_idx_if_start: 0,
                        });

                        DataflowOperand {
                            node: bb_idx,
                            output_idx: operand.find_index(ctx) as u32,
                        }
                    }
                })
                .collect::<SmallVec<[_; 2]>>();

            let (DataflowNode::Stmt {
                operands: node_operands,
                ..
            }
            | DataflowNode::Terminator {
                direct_operands: node_operands,
                ..
            }) = &mut graph.nodes[node_idx]
            else {
                unreachable!();
            };

            *node_operands = node_operands_init;
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

// === DataflowAnalysis === //

// TODO
