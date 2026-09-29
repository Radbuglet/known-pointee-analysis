// Pliron doesn't really have a dataflow analysis framework so I wrote one myself. It's terrible.
// I'm so so very sorry.

use std::{collections::VecDeque, mem, slice};

use derive_where::derive_where;
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
    pub struct DataflowBasicBlockArg = u32;
}

#[derive(Debug, Copy, Clone)]
pub enum DataflowSlotIdx {
    Effect(DataflowEffectIdx),
    State(DataflowStateIdx),
}

impl From<DataflowEffectIdx> for DataflowSlotIdx {
    fn from(value: DataflowEffectIdx) -> Self {
        Self::Effect(value)
    }
}

impl From<DataflowStateIdx> for DataflowSlotIdx {
    fn from(value: DataflowStateIdx) -> Self {
        Self::State(value)
    }
}

define_index_type! {
    pub struct DataflowEffectIdx = u32;
}

define_index_type! {
    pub struct DataflowStateIdx = u32;
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
    ($(,)?) => {
        // (recursion base case)
    };
    (
        copy $name:ident => $ty:ty : $ident:ident in $pat:pat
        $(, $($rest:tt)*)?
    ) => {
        paste::paste! {
            #[allow(unused)]
            impl DataflowGraphNode {
                pub fn [< as_ $name >](&self) -> Option<$ty> {
                    self.[< as_ $name _ref >]().copied()
                }

                pub fn [< unwrap_ $name >](&self) -> $ty {
                    self.[< as_ $name >]().unwrap()
                }
            }
        }

        make_conversions! {
            $name => $ty : $ident in $pat
            $(, $($rest)*)?
        }
    };
    (
        $name:ident => $ty:ty : $ident:ident in $pat:pat
        $(, $($rest:tt)*)?
    ) => {
        // Paste my beloved.
        paste::paste! {
            #[allow(unused)]
            impl DataflowGraphNode {
                pub fn [< as_ $name _ref >](&self) -> Option<&$ty> {
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

                pub fn [< unwrap_ $name _ref >](&self) -> &$ty {
                    self.[< as_ $name _ref >]().unwrap()
                }

                pub fn [< unwrap_ $name _mut >](&mut self) -> &mut $ty {
                    self.[< as_ $name _mut >]().unwrap()
                }
            }
        }

        $(
            make_conversions! {
                $($rest)*
            }
        )?
    };
}

make_conversions! {
    phi => DataflowGraphNodePhi : v in Self::Phi(v),
    stmt => DataflowGraphNodeStmt : v in Self::Stmt(v),
    terminator => DataflowGraphNodeTerminator : v in Self::Terminator(v),
    copy output_state => DataflowStateIdx : v in
        | Self::Phi(DataflowGraphNodePhi { output_state: v, .. })
        | Self::Stmt(DataflowGraphNodeStmt { output_state: Some(v), .. }),
    copy input_effect => DataflowEffectIdx : v in
        | Self::Stmt(DataflowGraphNodeStmt { input_effect: v, .. })
        | Self::Terminator(DataflowGraphNodeTerminator { input_effect: v, .. }),
    copy operation => Ptr<Operation> : v in
        | Self::Stmt(DataflowGraphNodeStmt { operation: v, .. })
        | Self::Terminator(DataflowGraphNodeTerminator { operation: v, .. }),
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
        for node_idx in graph.node_defs.indices() {
            match &graph.node_defs[node_idx] {
                DataflowGraphNode::Phi(DataflowGraphNodePhi { .. }) => {
                    // (connected in terminators)
                }
                DataflowGraphNode::Stmt(DataflowGraphNodeStmt { operation, .. }) => {
                    let operation_r = operation.deref(ctx);
                    let operation_succ = operation_r.get_next().unwrap();

                    // Determine `output_effect`
                    let init_output_effect =
                        graph.node_defs[graph.op_map[&operation_succ]].unwrap_input_effect();

                    // Determine `input_states`
                    let init_input_states = operation_r
                        .operands()
                        .map(|value| graph.lookup_value_state(ctx, value))
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
                        .map(|value| graph.lookup_value_state(ctx, value))
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
                            graph.node_defs[graph.op_map[&first_op]].unwrap_input_effect()
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
                            let src_state = graph.lookup_value_state(ctx, src_value);

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

    pub fn lookup_value_state(&self, ctx: &Context, value: Value) -> DataflowStateIdx {
        match value.defining_entity() {
            DefiningEntity::Op(op) => self.node_defs[self.op_map[&op]].unwrap_output_state(),
            DefiningEntity::Block(bb) => {
                let phi_node = self.bb_map[&bb][value.find_index(ctx)];
                self.node_defs[phi_node].unwrap_output_state()
            }
        }
    }

    pub fn is_input_state(&self, ctx: &Context, state: DataflowStateIdx) -> bool {
        match &self.node_defs[self.state_defs[state].defined_by] {
            DataflowGraphNode::Phi(DataflowGraphNodePhi { basic_block, .. })
                if basic_block.deref(ctx).get_prev().is_none() =>
            {
                true
            }
            _ => false,
        }
    }

    pub fn is_input_effect(&self, ctx: &Context, effect: DataflowEffectIdx) -> bool {
        let op = self.node_defs[self.effect_defs[effect].input_to]
            .as_operation()
            .unwrap()
            .deref(ctx);

        if op.get_prev().is_some() {
            // (not the first operation)
            return false;
        }

        if op
            .get_parent_block()
            .unwrap()
            .deref(ctx)
            .get_prev()
            .is_some()
        {
            // (not the entry block)
            return false;
        }

        true
    }

    pub fn affected_nodes(&self, slot: DataflowSlotIdx) -> &[DataflowNodeIdx] {
        match slot {
            DataflowSlotIdx::Effect(idx) => slice::from_ref(&self.effect_defs[idx].input_to),
            DataflowSlotIdx::State(idx) => &self.state_defs[idx].input_to,
        }
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

#[derive_where(Default)]
pub struct DataflowScratch<E, S> {
    work_list: VecDeque<DataflowSlotIdx>,
    effects: IndexVec<DataflowEffectIdx, DataflowScratchEntry<E>>,
    states: IndexVec<DataflowStateIdx, DataflowScratchEntry<S>>,
}

struct DataflowScratchEntry<T> {
    state: T,
    dirty: bool,
}

impl<E, S> DataflowScratch<E, S> {
    fn mark_dirty(&mut self, target: DataflowSlotIdx) {
        match target {
            DataflowSlotIdx::Effect(idx) => {
                if !mem::replace(&mut self.effects[idx].dirty, true) {
                    self.work_list.push_back(target);
                }
            }
            DataflowSlotIdx::State(idx) => {
                if !mem::replace(&mut self.states[idx].dirty, true) {
                    self.work_list.push_back(target);
                }
            }
        }
    }
}

pub trait DataflowAnalysis<'c>: Sized {
    type Effect: DataflowLattice<Self>;
    type State: DataflowLattice<Self>;

    fn ctx(&self) -> &'c Context;

    fn graph(&self) -> &'c DataflowGraph;

    fn run(&mut self, scratch: &mut DataflowScratch<Self::Effect, Self::State>) {
        let ctx = self.ctx();
        let graph = self.graph();

        // Setup initial states
        scratch.work_list.clear();

        scratch.effects.clear();
        scratch
            .effects
            .extend(graph.effect_defs.indices().map(|effect| {
                let is_input = graph.is_input_effect(ctx, effect);

                if is_input {
                    scratch.work_list.push_back(effect.into());

                    DataflowScratchEntry {
                        state: Self::Effect::top(self),
                        dirty: false,
                    }
                } else {
                    DataflowScratchEntry {
                        state: Self::Effect::bot(self),
                        dirty: false,
                    }
                }
            }));

        scratch.states.clear();
        scratch
            .states
            .extend(graph.state_defs.indices().map(|state| {
                let is_input = graph.is_input_state(ctx, state);

                if is_input {
                    scratch.work_list.push_back(state.into());

                    DataflowScratchEntry {
                        state: Self::State::top(self),
                        dirty: false,
                    }
                } else {
                    DataflowScratchEntry {
                        state: Self::State::bot(self),
                        dirty: false,
                    }
                }
            }));

        // Run dataflow
        while let Some(affected_slot) = scratch.work_list.pop_front() {
            for &affected_node in graph.affected_nodes(affected_slot) {
                match &graph.node_defs[affected_node] {
                    DataflowGraphNode::Phi(DataflowGraphNodePhi {
                        basic_block: _,
                        basic_block_arg: _,
                        input_states,
                        output_state,
                    }) => {
                        Self::State::reset_bot(self, &mut scratch.states[*output_state].state);

                        for &input_state in input_states {
                            let [output_state, input_state] = scratch
                                .states
                                .as_raw_slice_mut()
                                .get_disjoint_mut([output_state.index(), input_state.index()])
                                .unwrap();

                            Self::State::join(self, &mut output_state.state, &input_state.state);
                        }

                        scratch.mark_dirty((*output_state).into());
                    }
                    DataflowGraphNode::Stmt(_) => todo!(),
                    DataflowGraphNode::Terminator(_) => todo!(),
                }
            }
        }
    }
}

pub trait DataflowLattice<D> {
    fn bot(df: &mut D) -> Self;

    fn top(df: &mut D) -> Self;

    fn reset_bot(df: &mut D, target: &mut Self);

    fn join(df: &mut D, target: &mut Self, other: &Self) -> bool;
}
