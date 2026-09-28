use index_vec::{IndexVec, define_index_type};
use pliron::{
    basic_block::BasicBlock,
    context::{Context, Ptr},
    linked_list::{ContainsLinkedList, LinkedList},
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
        output_consumers: SmallVec<[DataflowNodeIdx; 1]>,
    },
    /// A statement node which may produce a result and propagate that result elsewhere.
    Stmt {
        operation: Ptr<Operation>,

        /// `Stmt` and `StartOfBlock` nodes supplying our operand variable states.
        operands: SmallVec<[DataflowOperand; 2]>,

        /// `Stmt` and `StartOfBlock` nodes which consume our result.
        output_consumers: SmallVec<[DataflowOperand; 1]>,

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

struct DataflowResultConsumer {
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
                if op.deref(ctx).get_next() == None {
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

        // Link up nodes
        for node_idx in &mut graph.nodes.indices() {
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

                    let init_operands = operation
                        .deref(ctx)
                        .operands()
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

                                op_output_consumers.push(DataflowOperand {
                                    node: node_idx,
                                    output_idx: 0,
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

                                bb_output_consumers.push(node_idx);

                                DataflowOperand {
                                    node: bb_idx,
                                    output_idx: operand.find_index(ctx) as u32,
                                }
                            }
                        })
                        .collect::<SmallVec<[_; 2]>>();

                    let DataflowNode::Stmt { operands, .. } = &mut graph.nodes[node_idx] else {
                        unreachable!();
                    };

                    *operands = init_operands;
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
        todo!()
    }
}

// === DataflowAnalysis === //

// TODO
