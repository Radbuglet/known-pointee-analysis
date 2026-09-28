// Pliron doesn't really have a dataflow analysis framework so I wrote one myself. It's terrible.
// I'm so so very sorry.

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
    pub struct DataflowNodeIdx = u32;
}

define_index_type! {
    pub struct DataflowEffectIdx = u32;
}

define_index_type! {
    pub struct DataflowStateIdx = u32;
}

define_index_type! {
    pub struct DataflowBasicBlockArg = u32;
}

#[derive(Debug, Clone)]
pub struct DataflowGraph {
    pub node_defs: IndexVec<DataflowNodeIdx, DataflowGraphNode>,
    pub effect_defs: IndexVec<DataflowEffectIdx, DataflowGraphEffect>,
    pub state_defs: IndexVec<DataflowStateIdx, DataflowGraphState>,
    pub op_map: FxHashMap<Ptr<Operation>, DataflowNodeIdx>,
    pub bb_map: FxHashMap<Ptr<BasicBlock>, SmallVec<[DataflowNodeIdx; 2]>>,
}

#[derive(Debug, Clone)]
pub enum DataflowGraphNode {
    Phi(DataflowGraphNodePhi),
    Stmt(DataflowGraphNodeStmt),
    Terminator(DataflowGraphNodeTerminator),
}

impl From<DataflowGraphNodePhi> for DataflowGraphNode {
    fn from(value: DataflowGraphNodePhi) -> Self {
        Self::Phi(value)
    }
}

impl From<DataflowGraphNodeStmt> for DataflowGraphNode {
    fn from(value: DataflowGraphNodeStmt) -> Self {
        Self::Stmt(value)
    }
}

impl From<DataflowGraphNodeTerminator> for DataflowGraphNode {
    fn from(value: DataflowGraphNodeTerminator) -> Self {
        Self::Terminator(value)
    }
}

macro_rules! make_conversions {
    (
        $($name:ident => $ty:ty : $ident:ident in $pat:pat),*
        $(,)?
    ) => {
        // Paste my beloved.
        paste::paste! {
            #[allow(unused)]
            impl DataflowGraphNode {
                $(
                    pub fn [< as_ $name >](&self) -> Option<&$ty> {
                        match self {
                            $pat => Some($ident),
                            _ => None,
                        }
                    }

                    pub fn [< as_ $name _mut >](&mut self) -> Option<&mut $ty> {
                        match self {
                            $pat => Some($ident),
                            _ => None,
                        }
                    }

                    pub fn [< unwrap_ $name >](&self) -> &$ty {
                        self.[< as_ $name >]().unwrap()
                    }

                    pub fn [< unwrap_ $name _mut >](&mut self) -> &mut $ty {
                        self.[< as_ $name _mut >]().unwrap()
                    }
                )*
            }
        }
    };
}

make_conversions! {
    phi => DataflowGraphNodePhi : v in Self::Phi(v),
    stmt => DataflowGraphNodeStmt : v in Self::Stmt(v),
    terminator => DataflowGraphNodeTerminator : v in Self::Terminator(v),
    output_state => DataflowStateIdx : v in
        | Self::Phi(DataflowGraphNodePhi { output_state: v, .. })
        | Self::Stmt(DataflowGraphNodeStmt { output_state: Some(v), .. }),
    input_effect => DataflowEffectIdx : v in
        | Self::Stmt(DataflowGraphNodeStmt { input_effect: v, .. })
        | Self::Terminator(DataflowGraphNodeTerminator { input_effect: v, .. }),
}

#[derive(Debug, Clone)]
pub struct DataflowGraphNodePhi {
    pub basic_block: Ptr<BasicBlock>,
    pub basic_block_arg: DataflowBasicBlockArg,
    pub input_states: SmallVec<[DataflowStateIdx; 2]>,
    pub output_state: DataflowStateIdx,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphNodeStmt {
    pub operation: Ptr<Operation>,
    pub input_effect: DataflowEffectIdx,
    pub output_effect: DataflowEffectIdx,
    pub input_states: SmallVec<[DataflowStateIdx; 2]>,
    pub output_state: Option<DataflowStateIdx>,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphNodeTerminator {
    pub operation: Ptr<Operation>,
    pub input_effect: DataflowEffectIdx,
    pub input_states: SmallVec<[DataflowStateIdx; 2]>,
    pub output_effects: SmallVec<[DataflowEffectIdx; 2]>,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphEffect {
    pub input_to: DataflowNodeIdx,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphState {
    pub defined_by: DataflowNodeIdx,
    pub input_to: SmallVec<[DataflowNodeIdx; 1]>,
}

impl DataflowGraph {
    pub fn new(ctx: &Context, region: Ptr<Region>) -> Self {
        let mut graph = DataflowGraph {
            node_defs: IndexVec::default(),
            effect_defs: IndexVec::default(),
            state_defs: IndexVec::default(),
            op_map: FxHashMap::default(),
            bb_map: FxHashMap::default(),
        };

        // Create placeholder phi nodes, statements, and terminators.
        for basic_block in region.deref(ctx).iter(ctx) {
            for operation in basic_block.deref(ctx).iter(ctx) {
                let operation_r = operation.deref(ctx);

                let input_effect = graph.effect_defs.push(DataflowGraphEffect {
                    input_to: graph.node_defs.next_idx(),
                });

                if operation_r.get_next().is_some() {
                    let output_state = match operation_r.get_num_results() {
                        0 => None,
                        1 => Some(graph.state_defs.push(DataflowGraphState {
                            defined_by: graph.node_defs.next_idx(),
                            input_to: SmallVec::new(),
                        })),
                        _ => unreachable!(),
                    };

                    let node_idx = graph.node_defs.push(
                        DataflowGraphNodeStmt {
                            operation,
                            input_effect: input_effect,
                            output_effect: DataflowEffectIdx::from_usize(
                                DataflowEffectIdx::MAX_INDEX,
                            ),
                            input_states: SmallVec::new(),
                            output_state,
                        }
                        .into(),
                    );

                    graph.op_map.insert(operation, node_idx);
                } else {
                    let node_idx = graph.node_defs.push(
                        DataflowGraphNodeTerminator {
                            operation,
                            input_effect,
                            input_states: SmallVec::new(),
                            output_effects: SmallVec::new(),
                        }
                        .into(),
                    );

                    graph.op_map.insert(operation, node_idx);
                }
            }

            let argument_phi_nodes = (0..basic_block.deref(ctx).get_num_arguments())
                .map(|basic_block_arg| {
                    let output_state = graph.state_defs.push(DataflowGraphState {
                        defined_by: graph.node_defs.next_idx(),
                        input_to: SmallVec::new(),
                    });

                    graph.node_defs.push(
                        DataflowGraphNodePhi {
                            basic_block,
                            basic_block_arg: DataflowBasicBlockArg::from_usize(basic_block_arg),
                            input_states: SmallVec::new(),
                            output_state,
                        }
                        .into(),
                    )
                })
                .collect();

            graph.bb_map.insert(basic_block, argument_phi_nodes);
        }

        // Connect up everything.
        fn lookup_state(graph: &DataflowGraph, ctx: &Context, value: Value) -> DataflowStateIdx {
            match value.defining_entity() {
                DefiningEntity::Op(op) => *graph.node_defs[graph.op_map[&op]].unwrap_output_state(),
                DefiningEntity::Block(bb) => {
                    let phi_node = graph.bb_map[&bb][value.find_index(ctx)];
                    *graph.node_defs[phi_node].unwrap_output_state()
                }
            }
        }

        for node_idx in graph.node_defs.indices() {
            match &graph.node_defs[node_idx] {
                DataflowGraphNode::Phi(DataflowGraphNodePhi { .. }) => {
                    // (connected in terminators)
                }
                DataflowGraphNode::Stmt(DataflowGraphNodeStmt { operation, .. }) => {
                    let operation_r = operation.deref(ctx);
                    let operation_succ = operation_r.get_next().unwrap();

                    // Determine `output_effect`
                    let init_output_effect = *graph.node_defs[graph.op_map[&operation_succ]]
                        .as_input_effect()
                        .unwrap();

                    // Determine `input_states`
                    let init_input_states = operation_r
                        .operands()
                        .map(|value| lookup_state(&graph, ctx, value))
                        .collect::<SmallVec<[DataflowStateIdx; 2]>>();

                    for &input_state in &init_input_states {
                        graph.state_defs[input_state].input_to.push(node_idx);
                    }

                    // Write out states
                    let DataflowGraphNodeStmt {
                        operation: _,
                        input_effect: _, // (already init)
                        output_effect,
                        input_states,
                        output_state: _, // (already init)
                    } = graph.node_defs[node_idx].unwrap_stmt_mut();

                    *output_effect = init_output_effect;
                    *input_states = init_input_states;
                }
                DataflowGraphNode::Terminator(DataflowGraphNodeTerminator {
                    operation, ..
                }) => {
                    let operation = *operation;
                    let operation_r = operation.deref(ctx);

                    // Determine `input_states`
                    // TODO: possibly truncate states that are only used in forwarding
                    let init_input_states = operation_r
                        .operands()
                        .map(|value| lookup_state(&graph, ctx, value))
                        .collect::<SmallVec<[DataflowStateIdx; 2]>>();

                    for &input_state in &init_input_states {
                        graph.state_defs[input_state].input_to.push(node_idx);
                    }

                    // Determine `output_effects`
                    let init_output_effects = operation_r
                        .successors()
                        .into_iter()
                        .map(|bb| {
                            let first_op = bb.deref(ctx).get_head().unwrap();
                            *graph.node_defs[graph.op_map[&first_op]].unwrap_input_effect()
                        })
                        .collect::<SmallVec<[_; 2]>>();

                    // Write out states
                    let DataflowGraphNodeTerminator {
                        operation: _,
                        input_effect: _, // (already init)
                        input_states,
                        output_effects,
                    } = graph.node_defs[node_idx].unwrap_terminator_mut();

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

                            let DataflowGraphNodePhi {
                                input_states: dst_input_states,
                                ..
                            } = graph.node_defs[dst_node].unwrap_phi_mut();

                            dst_input_states.push(src_state);
                            graph.state_defs[src_state].input_to.push(dst_node);
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

// === DataflowAnalysis === //

pub trait DataflowAnalysis<'c>: Sized {
    type Effect: DataflowLattice<Self>;
    type State: DataflowLattice<Self>;

    fn ctx(&self) -> &'c Context;

    fn graph(&self) -> &'c DataflowGraph;

    fn run(&mut self) {
        struct Lattice<S> {
            state: S,
            dirty: bool,
        }

        let ctx = self.ctx();
        let graph = self.graph();

        let mut states = IndexVec::<DataflowStateIdx, Lattice<Self::State>>::from_iter([]);
        let mut effects = IndexVec::<DataflowStateIdx, Lattice<Self::Effect>>::default();
    }
}

pub trait DataflowLattice<D> {}
