// Pliron doesn't really have a dataflow analysis framework so I wrote one myself. It's terrible.
// I'm so so very sorry.

use std::{
    collections::VecDeque,
    fmt::{self, Display},
    marker::PhantomData,
    mem, slice,
};

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
    Value(DataflowValueIdx),
}

impl From<DataflowEffectIdx> for DataflowSlotIdx {
    fn from(value: DataflowEffectIdx) -> Self {
        Self::Effect(value)
    }
}

impl From<DataflowValueIdx> for DataflowSlotIdx {
    fn from(value: DataflowValueIdx) -> Self {
        Self::Value(value)
    }
}

define_index_type! {
    pub struct DataflowEffectIdx = u32;
}

define_index_type! {
    pub struct DataflowValueIdx = u32;
}

#[derive(Debug, Clone)]
pub struct DataflowGraph {
    pub region: Ptr<Region>,
    pub node_defs: IndexVec<DataflowNodeIdx, DataflowGraphNode>,
    pub effect_defs: IndexVec<DataflowEffectIdx, DataflowGraphEffect>,
    pub value_defs: IndexVec<DataflowValueIdx, DataflowGraphValue>,
    pub op_map: FxHashMap<Ptr<Operation>, DataflowNodeIdx>,
    pub bb_map: FxHashMap<Ptr<BasicBlock>, DataflowBb>,
}

#[derive(Debug, Clone)]
pub struct DataflowBb {
    pub effect_phi_node: DataflowNodeIdx,
    pub arg_phi_nodes: SmallVec<[DataflowNodeIdx; 2]>,
}

#[derive(Debug, Clone)]
pub enum DataflowGraphNode {
    EffectPhi(DataflowGraphNodeEffectPhi),
    ValuePhi(DataflowGraphNodeValuePhi),
    Stmt(DataflowGraphNodeStmt),
    Terminator(DataflowGraphNodeTerminator),
}

impl From<DataflowGraphNodeEffectPhi> for DataflowGraphNode {
    fn from(value: DataflowGraphNodeEffectPhi) -> Self {
        Self::EffectPhi(value)
    }
}

impl From<DataflowGraphNodeValuePhi> for DataflowGraphNode {
    fn from(value: DataflowGraphNodeValuePhi) -> Self {
        Self::ValuePhi(value)
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
    effect_phi => DataflowGraphNodeEffectPhi : v in Self::EffectPhi(v),
    value_phi => DataflowGraphNodeValuePhi : v in Self::ValuePhi(v),
    stmt => DataflowGraphNodeStmt : v in Self::Stmt(v),
    terminator => DataflowGraphNodeTerminator : v in Self::Terminator(v),
    copy output_value => DataflowValueIdx : v in
        | Self::ValuePhi(DataflowGraphNodeValuePhi { output_value: v, .. })
        | Self::Stmt(DataflowGraphNodeStmt { output_value: Some(v), .. }),
    copy input_effect => DataflowEffectIdx : v in
        | Self::Stmt(DataflowGraphNodeStmt { input_effect: v, .. })
        | Self::Terminator(DataflowGraphNodeTerminator { input_effect: v, .. }),
    copy operation => Ptr<Operation> : v in
        | Self::Stmt(DataflowGraphNodeStmt { operation: v, .. })
        | Self::Terminator(DataflowGraphNodeTerminator { operation: v, .. }),
}

#[derive(Debug, Clone)]
pub struct DataflowGraphNodeEffectPhi {
    pub basic_block: Ptr<BasicBlock>,
    pub input_effects: SmallVec<[DataflowEffectIdx; 2]>,
    pub output_effect: DataflowEffectIdx,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphNodeValuePhi {
    pub basic_block: Ptr<BasicBlock>,
    pub basic_block_arg: DataflowBasicBlockArg,
    pub input_values: SmallVec<[DataflowValueIdx; 2]>,
    pub output_value: DataflowValueIdx,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphNodeStmt {
    pub operation: Ptr<Operation>,
    pub input_effect: DataflowEffectIdx,
    pub output_effect: DataflowEffectIdx,
    pub input_values: SmallVec<[DataflowValueIdx; 2]>,
    pub output_value: Option<DataflowValueIdx>,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphNodeTerminator {
    pub operation: Ptr<Operation>,
    pub input_effect: DataflowEffectIdx,
    pub input_values: SmallVec<[DataflowValueIdx; 2]>,
    pub output_effects: SmallVec<[DataflowEffectIdx; 2]>,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphEffect {
    pub input_to: DataflowNodeIdx,
}

#[derive(Debug, Clone)]
pub struct DataflowGraphValue {
    pub defined_by: DataflowNodeIdx,
    pub input_to: SmallVec<[DataflowNodeIdx; 1]>,
}

impl DataflowGraph {
    pub fn new(ctx: &Context, region: Ptr<Region>) -> Self {
        let mut graph = DataflowGraph {
            region,
            node_defs: IndexVec::default(),
            effect_defs: IndexVec::default(),
            value_defs: IndexVec::default(),
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
                    let output_value = match operation_r.get_num_results() {
                        0 => None,
                        1 => Some(graph.value_defs.push(DataflowGraphValue {
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
                            input_values: SmallVec::new(),
                            output_value,
                        }
                        .into(),
                    );

                    graph.op_map.insert(operation, node_idx);
                } else {
                    let node_idx = graph.node_defs.push(
                        DataflowGraphNodeTerminator {
                            operation,
                            input_effect,
                            input_values: SmallVec::new(),
                            output_effects: SmallVec::new(),
                        }
                        .into(),
                    );

                    graph.op_map.insert(operation, node_idx);
                }
            }

            let effect_phi_node = graph.node_defs.push(
                DataflowGraphNodeEffectPhi {
                    basic_block,
                    input_effects: SmallVec::new(),
                    output_effect: graph.node_defs
                        [graph.op_map[&basic_block.deref(ctx).get_head().unwrap()]]
                        .unwrap_input_effect(),
                }
                .into(),
            );

            let arg_phi_nodes = (0..basic_block.deref(ctx).get_num_arguments())
                .map(|basic_block_arg| {
                    let output_value = graph.value_defs.push(DataflowGraphValue {
                        defined_by: graph.node_defs.next_idx(),
                        input_to: SmallVec::new(),
                    });

                    graph.node_defs.push(
                        DataflowGraphNodeValuePhi {
                            basic_block,
                            basic_block_arg: DataflowBasicBlockArg::from_usize(basic_block_arg),
                            input_values: SmallVec::new(),
                            output_value,
                        }
                        .into(),
                    )
                })
                .collect();

            graph.bb_map.insert(
                basic_block,
                DataflowBb {
                    effect_phi_node,
                    arg_phi_nodes,
                },
            );
        }

        // Connect up everything.
        for node_idx in graph.node_defs.indices() {
            match &graph.node_defs[node_idx] {
                DataflowGraphNode::EffectPhi(_) | DataflowGraphNode::ValuePhi(_) => {
                    // (connected in terminators)
                }
                DataflowGraphNode::Stmt(DataflowGraphNodeStmt { operation, .. }) => {
                    let operation_r = operation.deref(ctx);
                    let operation_succ = operation_r.get_next().unwrap();

                    // Determine `output_effect`
                    let init_output_effect =
                        graph.node_defs[graph.op_map[&operation_succ]].unwrap_input_effect();

                    // Determine `input_values`
                    let init_input_values = operation_r
                        .operands()
                        .map(|value| graph.lookup_value_idx(ctx, value))
                        .collect::<SmallVec<[DataflowValueIdx; 2]>>();

                    for &input_value in &init_input_values {
                        graph.value_defs[input_value].input_to.push(node_idx);
                    }

                    // Write out connections
                    let DataflowGraphNodeStmt {
                        operation: _,
                        input_effect: _, // (already init)
                        output_effect,
                        input_values,
                        output_value: _, // (already init)
                    } = graph.node_defs[node_idx].unwrap_stmt_mut();

                    *output_effect = init_output_effect;
                    *input_values = init_input_values;
                }
                DataflowGraphNode::Terminator(DataflowGraphNodeTerminator {
                    operation, ..
                }) => {
                    let operation = *operation;
                    let operation_r = operation.deref(ctx);

                    // Determine `input_values`
                    let init_input_values = operation_r
                        .operands()
                        .map(|value| graph.lookup_value_idx(ctx, value))
                        .collect::<SmallVec<[DataflowValueIdx; 2]>>();

                    for &input_value in &init_input_values {
                        graph.value_defs[input_value].input_to.push(node_idx);
                    }

                    // Determine `output_effects`
                    let init_output_effects = operation_r
                        .successors()
                        .into_iter()
                        .map(|bb| {
                            let effect_phi_node = graph.bb_map[&bb].effect_phi_node;
                            let effect = graph.effect_defs.push(DataflowGraphEffect {
                                input_to: effect_phi_node,
                            });

                            graph.node_defs[effect_phi_node]
                                .as_effect_phi_mut()
                                .unwrap()
                                .input_effects
                                .push(effect);

                            effect
                        })
                        .collect::<SmallVec<[_; 2]>>();

                    // Write out connections
                    let DataflowGraphNodeTerminator {
                        operation: _,
                        input_effect: _, // (already init)
                        input_values,
                        output_effects,
                    } = graph.node_defs[node_idx].unwrap_terminator_mut();

                    *input_values = init_input_values;
                    *output_effects = init_output_effects;

                    // Link up value phi node inputs.
                    if operation_r.get_num_successors() == 0 {
                        // Return and unreachable aren't considered branches.
                        continue;
                    }

                    let operation_b = Operation::get_op_dyn(operation, ctx);
                    let operation_b = op_cast::<dyn BranchOpInterface>(&*operation_b).unwrap();

                    for succ_idx in 0..operation_r.get_num_successors() {
                        let succ = operation_r.get_successor(succ_idx);
                        let arg_phi_nodes = &graph.bb_map[&succ].arg_phi_nodes;

                        for (src_value, &dst_node) in operation_b
                            .successor_operands(ctx, succ_idx)
                            .into_iter()
                            .zip(arg_phi_nodes)
                        {
                            let src_value_idx = graph.lookup_value_idx(ctx, src_value);

                            let DataflowGraphNodeValuePhi {
                                input_values: dst_input_values,
                                ..
                            } = graph.node_defs[dst_node].unwrap_value_phi_mut();

                            dst_input_values.push(src_value_idx);
                            graph.value_defs[src_value_idx].input_to.push(dst_node);
                        }
                    }
                }
            }
        }

        for node in &mut graph.node_defs {
            if let Some(phi) = node.as_effect_phi_mut() {
                phi.input_effects.sort();
                phi.input_effects.dedup();
                phi.input_effects.retain(|&mut v| v != phi.output_effect);
            }

            if let Some(phi) = node.as_value_phi_mut() {
                phi.input_values.sort();
                phi.input_values.dedup();
                phi.input_values.retain(|&mut v| v != phi.output_value);
            }
        }

        graph
    }

    pub fn lookup_value_idx(&self, ctx: &Context, value: Value) -> DataflowValueIdx {
        match value.defining_entity() {
            DefiningEntity::Op(op) => self.node_defs[self.op_map[&op]].unwrap_output_value(),
            DefiningEntity::Block(bb) => {
                let phi_node = self.bb_map[&bb].arg_phi_nodes[value.find_index(ctx)];
                self.node_defs[phi_node].unwrap_output_value()
            }
        }
    }

    pub fn is_input_value(&self, ctx: &Context, value: DataflowValueIdx) -> bool {
        match &self.node_defs[self.value_defs[value].defined_by] {
            DataflowGraphNode::ValuePhi(DataflowGraphNodeValuePhi { basic_block, .. })
                if basic_block.deref(ctx).get_prev().is_none() =>
            {
                true
            }
            _ => false,
        }
    }

    pub fn is_input_effect(&self, ctx: &Context, effect: DataflowEffectIdx) -> bool {
        match &self.node_defs[self.effect_defs[effect].input_to] {
            DataflowGraphNode::EffectPhi(DataflowGraphNodeEffectPhi { basic_block, .. })
                if basic_block.deref(ctx).get_prev().is_none() =>
            {
                true
            }
            _ => false,
        }
    }

    pub fn affected_nodes(&self, slot: DataflowSlotIdx) -> &[DataflowNodeIdx] {
        match slot {
            DataflowSlotIdx::Effect(idx) => slice::from_ref(&self.effect_defs[idx].input_to),
            DataflowSlotIdx::Value(idx) => &self.value_defs[idx].input_to,
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

pub struct DataflowScratch<E, V> {
    work_list: VecDeque<DataflowSlotIdx>,
    effects: IndexVec<DataflowEffectIdx, DataflowSlot<E>>,
    values: IndexVec<DataflowValueIdx, DataflowSlot<V>>,
    vec_of_ptrs_1: VecOfPtrScratch,
    vec_of_ptrs_2: VecOfPtrScratch,
    borrows: DisjointBorrowsScratch,
}

impl<E, V> Default for DataflowScratch<E, V> {
    fn default() -> Self {
        Self {
            work_list: VecDeque::default(),
            effects: IndexVec::default(),
            values: IndexVec::default(),
            vec_of_ptrs_1: VecOfPtrScratch::default(),
            vec_of_ptrs_2: VecOfPtrScratch::default(),
            borrows: DisjointBorrowsScratch::default(),
        }
    }
}

impl<E, V> DataflowScratch<E, V> {
    pub fn effect(&self, effect: DataflowEffectIdx) -> &E {
        &self.effects[effect].lattice
    }

    pub fn value(&self, effect: DataflowValueIdx) -> &V {
        &self.values[effect].lattice
    }

    fn take_from_work_list(&mut self) -> Option<DataflowSlotIdx> {
        let elem = self.work_list.pop_front()?;

        match elem {
            DataflowSlotIdx::Effect(idx) => {
                self.effects[idx].in_work_list = false;
            }
            DataflowSlotIdx::Value(idx) => {
                self.values[idx].in_work_list = false;
            }
        }

        Some(elem)
    }

    fn mark_dirty(&mut self, target: DataflowSlotIdx) {
        match target {
            DataflowSlotIdx::Effect(idx) => {
                if !mem::replace(&mut self.effects[idx].in_work_list, true) {
                    self.work_list.push_back(target);
                }
            }
            DataflowSlotIdx::Value(idx) => {
                if !mem::replace(&mut self.values[idx].in_work_list, true) {
                    self.work_list.push_back(target);
                }
            }
        }
    }

    fn process_user_mark(&mut self, target: DataflowSlotIdx) {
        match target {
            DataflowSlotIdx::Effect(idx) => {
                if !mem::take(&mut self.effects[idx].user_marked_dirty) {
                    return;
                }
            }
            DataflowSlotIdx::Value(idx) => {
                if !mem::take(&mut self.values[idx].user_marked_dirty) {
                    return;
                }
            }
        }

        self.mark_dirty(target);
    }
}

pub trait DataflowAnalysis<'c>: Sized {
    type Effect;
    type Value;

    fn ctx(&self) -> &'c Context;

    fn graph(&self) -> &'c DataflowGraph;

    fn init_effect(&mut self, is_input: bool) -> Self::Effect;

    fn init_value(&mut self, is_input: bool) -> Self::Value;

    fn trans_effect_phi(
        &mut self,
        input_values: &[&Self::Effect],
        output_value: &mut DataflowSlot<Self::Effect>,
    );

    fn trans_value_phi(
        &mut self,
        input_values: &[&Self::Value],
        output_value: &mut DataflowSlot<Self::Value>,
    );

    fn trans_stmt(
        &mut self,
        operation: Ptr<Operation>,
        input_effect: &Self::Effect,
        output_effect: &mut DataflowSlot<Self::Effect>,
        input_values: &[&Self::Value],
        output_value: Option<&mut DataflowSlot<Self::Value>>,
    );

    fn trans_terminator(
        &mut self,
        operation: Ptr<Operation>,
        input_effect: &Self::Effect,
        input_values: &[&Self::Value],
        output_effects: &mut [&mut DataflowSlot<Self::Effect>],
    );

    fn run(&mut self, scratch: &mut DataflowScratch<Self::Effect, Self::Value>) {
        let ctx = self.ctx();
        let graph = self.graph();

        // Setup scratch
        scratch.work_list.clear();

        scratch.effects.clear();
        scratch
            .effects
            .extend(graph.effect_defs.indices().map(|effect| {
                let is_input = graph.is_input_effect(ctx, effect);

                scratch.work_list.push_back(effect.into());

                DataflowSlot {
                    lattice: self.init_effect(is_input),
                    user_marked_dirty: false,
                    in_work_list: true,
                }
            }));

        scratch.values.clear();
        scratch
            .values
            .extend(graph.value_defs.indices().map(|value| {
                let is_input = graph.is_input_value(ctx, value);

                scratch.work_list.push_back(value.into());

                DataflowSlot {
                    lattice: self.init_value(is_input),
                    user_marked_dirty: false,
                    in_work_list: true,
                }
            }));

        // Run dataflow
        while let Some(affected_slot) = scratch.take_from_work_list() {
            for &affected_node in graph.affected_nodes(affected_slot) {
                match &graph.node_defs[affected_node] {
                    DataflowGraphNode::EffectPhi(DataflowGraphNodeEffectPhi {
                        basic_block: _,
                        input_effects: input_effect_indices,
                        output_effect: output_effect_idx,
                    }) => {
                        let mut effect_borrows =
                            scratch.borrows.builder(scratch.effects.as_raw_slice_mut());

                        let input_effects = scratch.vec_of_ptrs_1.build(
                            input_effect_indices
                                .iter()
                                .map(|idx| &effect_borrows.get(idx.index()).lattice),
                        );
                        let output_effect = effect_borrows.get_mut(output_effect_idx.index());

                        self.trans_effect_phi(input_effects, output_effect);

                        scratch.process_user_mark((*output_effect_idx).into());
                    }
                    DataflowGraphNode::ValuePhi(DataflowGraphNodeValuePhi {
                        basic_block: _,
                        basic_block_arg: _,
                        input_values: input_value_indices,
                        output_value: output_value_idx,
                    }) => {
                        let mut value_borrows =
                            scratch.borrows.builder(scratch.values.as_raw_slice_mut());

                        let input_values = scratch.vec_of_ptrs_1.build(
                            input_value_indices
                                .iter()
                                .map(|idx| &value_borrows.get(idx.index()).lattice),
                        );
                        let output_value = value_borrows.get_mut(output_value_idx.index());

                        self.trans_value_phi(input_values, output_value);

                        scratch.process_user_mark((*output_value_idx).into());
                    }
                    DataflowGraphNode::Stmt(DataflowGraphNodeStmt {
                        operation,
                        input_effect: input_effect_idx,
                        output_effect: output_effect_idx,
                        input_values: input_value_indices,
                        output_value: output_value_index,
                    }) => {
                        let [
                            &mut DataflowSlot {
                                lattice: ref input_effect,
                                ..
                            },
                            output_effect,
                        ] = scratch
                            .effects
                            .as_raw_slice_mut()
                            .get_disjoint_mut([input_effect_idx.index(), output_effect_idx.index()])
                            .unwrap();

                        let mut value_borrows =
                            scratch.borrows.builder(scratch.values.as_raw_slice_mut());

                        let input_values = scratch.vec_of_ptrs_1.build(
                            input_value_indices
                                .iter()
                                .map(|idx| &value_borrows.get(idx.index()).lattice),
                        );

                        let output_value =
                            output_value_index.map(|idx| value_borrows.get_mut(idx.index()));

                        self.trans_stmt(
                            *operation,
                            input_effect,
                            output_effect,
                            input_values,
                            output_value,
                        );

                        scratch.process_user_mark((*output_effect_idx).into());

                        if let Some(output_value_index) = output_value_index {
                            scratch.process_user_mark((*output_value_index).into());
                        }
                    }
                    DataflowGraphNode::Terminator(DataflowGraphNodeTerminator {
                        operation,
                        input_effect: input_effect_index,
                        input_values: input_value_indices,
                        output_effects: output_effect_indices,
                    }) => {
                        let mut effect_borrows =
                            scratch.borrows.builder(scratch.effects.as_raw_slice_mut());

                        let input_effect = &effect_borrows.get(input_effect_index.index()).lattice;
                        let input_values = scratch.vec_of_ptrs_1.build(
                            input_value_indices
                                .iter()
                                .map(|&idx| &scratch.values[idx].lattice),
                        );

                        let output_effects = scratch.vec_of_ptrs_2.build(
                            output_effect_indices
                                .iter()
                                .map(|idx| effect_borrows.get_mut(idx.index())),
                        );

                        self.trans_terminator(
                            *operation,
                            input_effect,
                            input_values,
                            output_effects,
                        );

                        for &index in output_effect_indices {
                            scratch.process_user_mark(index.into());
                        }
                    }
                }
            }
        }
    }
}

// === Pretty === //

mod pretty {
    use std::fmt;

    use pliron::{
        common_traits::Named as _,
        context::Context,
        irfmt::printers::{iter_with_sep, op::typed_symb_op_header},
        linked_list::ContainsLinkedList as _,
        operation::Operation,
        printable::{ListSeparator, Printable as _},
        r#type::Typed as _,
    };
    use pliron_llvm::ops::FuncOp;

    use crate::dataflow::DataflowGraphNodeStmt;

    use super::{DataflowGraph, DataflowScratch};

    pub struct DataflowFactPretty<'a, E, V> {
        pub ctx: &'a Context,
        pub graph: &'a DataflowGraph,
        pub scratch: &'a DataflowScratch<E, V>,
        pub is_interesting_effect: &'a dyn Fn(&'a E) -> bool,
        pub is_interesting_value: &'a dyn Fn(&'a V) -> bool,
    }

    impl<E, V> fmt::Display for DataflowFactPretty<'_, E, V>
    where
        E: fmt::Debug,
        V: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self {
                ctx,
                graph,
                scratch,
                is_interesting_effect,
                is_interesting_value,
            } = self;

            writeln!(
                f,
                "{}",
                typed_symb_op_header(
                    &Operation::get_op::<FuncOp>(self.graph.region.deref(ctx).get_parent_op(), ctx)
                        .unwrap(),
                )
                .disp(ctx)
            )?;

            for bb in graph.region.deref(ctx).iter(ctx) {
                let bb_r = bb.deref(ctx);

                writeln!(
                    f,
                    "    ^{}({})",
                    bb_r.unique_name(ctx),
                    iter_with_sep(
                        bb_r.arguments().map(|arg| {
                            format!("{}: {}", arg.disp(ctx), arg.get_type(ctx).disp(ctx))
                        }),
                        ListSeparator::CharSpace(',')
                    )
                    .disp(ctx),
                )?;

                {
                    let effect = scratch.effect(
                        graph.node_defs[graph.bb_map[&bb].effect_phi_node]
                            .unwrap_effect_phi_ref()
                            .output_effect,
                    );

                    if is_interesting_effect(effect) {
                        writeln!(f, "    -> [!] effects: {effect:?}")?;
                    }
                }

                for (idx, argument) in bb_r.arguments().enumerate() {
                    let value = scratch.value(
                        graph.node_defs[graph.bb_map[&bb].arg_phi_nodes[idx]]
                            .unwrap_value_phi_ref()
                            .output_value,
                    );

                    if !is_interesting_value(value) {
                        continue;
                    }

                    writeln!(f, "    -> [!] {}: {value:?}", argument.disp(ctx),)?;
                }

                writeln!(f)?;

                for stmt in bb_r.iter(ctx) {
                    writeln!(f, "        {}", stmt.disp(ctx))?;

                    if let Some(output) = graph.node_defs[graph.op_map[&stmt]].as_output_value()
                        && let output = scratch.value(output)
                        && is_interesting_value(output)
                    {
                        writeln!(f, "        -> [!] result: {output:?}")?;
                    }

                    if let Some(DataflowGraphNodeStmt { output_effect, .. }) =
                        graph.node_defs[graph.op_map[&stmt]].as_stmt_ref()
                        && let output_effect = scratch.effect(*output_effect)
                        && is_interesting_effect(output_effect)
                    {
                        writeln!(f, "        -> [!] effects: {output_effect:?}")?;
                    }

                    writeln!(f)?;
                }

                writeln!(f)?;
            }

            Ok(())
        }
    }
}

pub fn dataflow_pretty<'a, E, V>(
    ctx: &'a Context,
    graph: &'a DataflowGraph,
    scratch: &'a DataflowScratch<E, V>,
    is_interesting_effect: &'a dyn Fn(&'a E) -> bool,
    is_interesting_value: &'a dyn Fn(&'a V) -> bool,
) -> Box<dyn Display + 'a>
where
    E: fmt::Debug,
    V: fmt::Debug,
{
    Box::new(pretty::DataflowFactPretty {
        ctx,
        graph,
        scratch,
        is_interesting_effect,
        is_interesting_value,
    })
}

// === DataflowSlot === //

pub struct DataflowSlot<T> {
    lattice: T,
    user_marked_dirty: bool,
    in_work_list: bool,
}

impl<T> DataflowSlot<T> {
    pub fn value(&self) -> &T {
        &self.lattice
    }

    pub fn value_mut_marked(&mut self) -> &mut T {
        self.user_marked_dirty = true;
        &mut self.lattice
    }

    pub fn value_mut_unmarked(&mut self) -> &mut T {
        &mut self.lattice
    }

    pub fn set_value(&mut self, value: T)
    where
        T: Eq,
    {
        self.maybe_mark_dirty(self.lattice != value);
        self.lattice = value;
    }

    pub fn set_value_ref(&mut self, value: &T)
    where
        T: Eq + Clone,
    {
        self.maybe_mark_dirty(self.lattice != *value);
        self.lattice = value.clone();
    }

    pub fn mark_dirty(&mut self) {
        self.user_marked_dirty = true;
    }

    pub fn maybe_mark_dirty(&mut self, is_dirty: bool) {
        self.user_marked_dirty |= is_dirty;
    }
}

// === Helpers === //

#[derive(Default)]
struct VecOfPtrScratch {
    buffer: Vec<*mut ()>,
}

unsafe impl Send for VecOfPtrScratch {}
unsafe impl Sync for VecOfPtrScratch {}

impl VecOfPtrScratch {
    pub fn builder<'i, T: SizedPointer>(&'i mut self) -> VecOfPtrScratchBuilder<'i, T> {
        self.buffer.clear();

        VecOfPtrScratchBuilder {
            _ty: PhantomData,
            buffer: &mut self.buffer,
        }
    }

    pub fn build<T: SizedPointer>(&mut self, iter: impl IntoIterator<Item = T>) -> &mut [T] {
        let mut builder = self.builder();
        builder.extend(iter);
        builder.finish()
    }
}

struct VecOfPtrScratchBuilder<'a, T: SizedPointer> {
    _ty: PhantomData<Vec<T>>,
    buffer: &'a mut Vec<*mut ()>,
}

impl<'a, T: SizedPointer> VecOfPtrScratchBuilder<'a, T> {
    fn finish(self) -> &'a mut [T] {
        let buffer = self.buffer.as_mut_slice() as *mut [*mut ()];

        unsafe { slice::from_raw_parts_mut(buffer.cast::<()>().cast::<T>(), buffer.len()) }
    }
}

impl<'a, T: SizedPointer> Extend<T> for VecOfPtrScratchBuilder<'a, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.buffer.extend(iter.into_iter().map(|v| v.as_ptr()));
    }
}

unsafe trait SizedPointer {
    fn as_ptr(self) -> *mut ();
}

unsafe impl<'a, T> SizedPointer for &'a T {
    fn as_ptr(self) -> *mut () {
        (self as *const T).cast::<()>().cast_mut()
    }
}

unsafe impl<'a, T> SizedPointer for &'a mut T {
    fn as_ptr(self) -> *mut () {
        (self as *mut T).cast()
    }
}

#[derive(Default)]
struct DisjointBorrowsScratch {
    borrowed_cells: Vec<BorrowState>,
    undo_set: Vec<usize>,
}

#[derive(Copy, Clone)]
enum BorrowState {
    None,
    Mut,
    Ref,
}

impl DisjointBorrowsScratch {
    fn builder<'s, 'a, T>(&'s mut self, target: &'a mut [T]) -> DisjointBorrowsBuilder<'s, 'a, T> {
        for idx in self.undo_set.drain(..) {
            self.borrowed_cells[idx] = BorrowState::None;
        }

        if self.borrowed_cells.len() < target.len() {
            self.borrowed_cells.resize(target.len(), BorrowState::None);
        }

        DisjointBorrowsBuilder {
            _ty: PhantomData,
            scratch: self,
            target,
        }
    }
}

struct DisjointBorrowsBuilder<'s, 'a, T> {
    _ty: PhantomData<&'a mut [T]>,
    scratch: &'s mut DisjointBorrowsScratch,
    target: *mut [T],
}

impl<'s, 'a, T> DisjointBorrowsBuilder<'s, 'a, T> {
    fn get(&mut self, index: usize) -> &'a T {
        assert!(index < self.target.len());

        let state = &mut self.scratch.borrowed_cells[index];

        match state {
            BorrowState::None | BorrowState::Ref => {
                *state = BorrowState::Ref;
            }
            BorrowState::Mut => {
                panic!("invalid immutable borrow");
            }
        }

        self.scratch.undo_set.push(index);

        unsafe { &*self.target.cast::<T>().add(index) }
    }

    fn get_mut(&mut self, index: usize) -> &'a mut T {
        assert!(index < self.target.len());

        let state = &mut self.scratch.borrowed_cells[index];

        match state {
            BorrowState::None => {
                *state = BorrowState::Mut;
            }
            BorrowState::Mut | BorrowState::Ref => {
                panic!("invalid mutable borrow");
            }
        }

        self.scratch.undo_set.push(index);

        unsafe { &mut *self.target.cast::<T>().add(index) }
    }
}
