use core::fmt;
use std::rc::Rc;

use pliron::{
    attribute::{Attribute, attr_cast},
    builtin::{
        attr_interfaces::TypedAttrInterface,
        attributes::IntegerAttr,
        op_interfaces::{AtMostOneRegionInterface as _, OneResultInterface},
        types::IntegerType,
    },
    context::{Context, Ptr},
    graph::walkers::{self, WalkConfig},
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    result::Error as PlironError,
    r#type::Typed,
    value::Value,
};
use pliron_llvm::{
    ops::{BrOp, CondBrOp, ConstantOp, FuncOp, LoadOp, StoreOp, TruncOp},
    types::PointerType,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::dataflow::{DataflowAnalysis, DataflowGraph, DataflowScratch, DataflowSlot};

// === Driver === //

pub struct PointeeConstantsFacts {
    pub facts: Option<DataflowScratch<PointeeEffectLattice, OptimisticScalar>>,
}

impl Analysis for PointeeConstantsFacts {
    fn name(&self) -> &str {
        "pointee constants"
    }

    fn compute(
        raw_op: Ptr<Operation>,
        ctx: &Context,
        analyses: &mut AnalysisManager,
    ) -> Result<Self, PlironError> {
        let Some(op) = Operation::get_op::<FuncOp>(raw_op, ctx) else {
            return Ok(Self { facts: None });
        };

        if op.get_region(ctx).is_none() {
            return Ok(Self { facts: None });
        }

        let graph = analyses.get_analysis::<DataflowGraph>(raw_op, ctx).unwrap();

        let mut analysis = MeowAnalysis {
            ctx,
            graph: &graph,
            pointers: FxHashMap::default(),
        };

        analysis.discover_ptrs_in_op(raw_op);

        let mut scratch = DataflowScratch::default();

        analysis.run(&mut scratch);

        Ok(Self {
            facts: Some(scratch),
        })
    }
}

// === Analysis === //

pub struct MeowAnalysis<'a> {
    ctx: &'a Context,
    graph: &'a DataflowGraph,
    pointers: FxHashMap<Value, PtrIdx>,
}

impl MeowAnalysis<'_> {
    pub fn discover_ptrs_in_op(&mut self, op: Ptr<Operation>) {
        let ctx = self.ctx();

        walkers::uninterruptible::immutable::walk_op(
            ctx,
            self,
            &WalkConfig::default(),
            op,
            |ctx, analysis, node| match node {
                walkers::IRNode::Operation(node) => {
                    for value in node.deref(ctx).results() {
                        analysis.visit_value_for_ptrs(value);
                    }
                }
                walkers::IRNode::BasicBlock(node) => {
                    for value in node.deref(ctx).arguments() {
                        analysis.visit_value_for_ptrs(value);
                    }
                }
                walkers::IRNode::Region(_) => {
                    // (ignored)
                }
            },
        );
    }

    fn visit_value_for_ptrs(&mut self, value: Value) {
        let ctx = self.ctx();

        if value
            .get_type(ctx)
            .deref(ctx)
            .downcast_ref::<PointerType>()
            .is_some()
        {
            let idx = PtrIdx(self.pointers.len() as u32);
            self.pointers.insert(value, idx);
        }
    }
}

impl<'a> DataflowAnalysis<'a> for MeowAnalysis<'a> {
    type Effect = PointeeEffectLattice;
    type Value = OptimisticScalar;

    fn ctx(&self) -> &'a Context {
        self.ctx
    }

    fn graph(&self) -> &'a DataflowGraph {
        self.graph
    }

    fn init_effect(&mut self, _is_input: bool) -> Self::Effect {
        PointeeEffectLattice::default()
    }

    fn init_value(&mut self, _is_input: bool) -> Self::Value {
        OptimisticScalar::default()
    }

    fn trans_effect_phi(
        &mut self,
        input_values: &[&Self::Effect],
        output_value: &mut DataflowSlot<Self::Effect>,
    ) {
        let Some((&first, remainder)) = input_values.split_first() else {
            return;
        };

        let mut target = first.clone();

        for &other in remainder {
            target.join(other);
        }

        output_value.set_value(target);
    }

    fn trans_value_phi(
        &mut self,
        input_states: &[&Self::Value],
        output_state: &mut DataflowSlot<Self::Value>,
    ) {
        let Some((&first, remainder)) = input_states.split_first() else {
            return;
        };

        let mut target = first.clone();

        for &other in remainder {
            target.join(other);
        }

        output_state.set_value(target);
    }

    fn trans_stmt(
        &mut self,
        operation: Ptr<Operation>,
        input_effect: &Self::Effect,
        output_effect: &mut DataflowSlot<Self::Effect>,
        input_states: &[&Self::Value],
        output_state: Option<&mut DataflowSlot<Self::Value>>,
    ) {
        let ctx = self.ctx();

        // Constant
        if let Some(operation) = Operation::get_op::<ConstantOp>(operation, ctx)
            && let Some(constant) =
                (&*operation.get_value(ctx) as &dyn Attribute).downcast_ref::<IntegerAttr>()
        {
            output_effect.set_value_ref(input_effect);
            output_state
                .unwrap()
                .set_value(OptimisticScalar::Known(constant.value().to_u64()));

            return;
        }

        // Truncate
        if let Some(operation) = Operation::get_op::<TruncOp>(operation, ctx)
            && let result_ty = operation.result_type(ctx)
            && let Some(int_ty) = result_ty.deref(ctx).downcast_ref::<IntegerType>()
        {
            output_effect.set_value_ref(input_effect);

            let mut new_output_state = input_states[0].clone();
            new_output_state.map(|value| value & ((1u64 << int_ty.width()) - 1));
            output_state.unwrap().set_value(new_output_state);

            return;
        }

        // Store
        if let Some(operation) = Operation::get_op::<StoreOp>(operation, ctx) {
            let ptr = self.pointers[&operation.get_operand_address(ctx)];
            let mut new_output_effect = input_effect.clone();
            new_output_effect.write(ptr, input_states[0]);
            output_effect.set_value(new_output_effect);

            assert!(output_state.is_none());

            return;
        }

        // Load
        if let Some(operation) = Operation::get_op::<LoadOp>(operation, ctx) {
            let ptr = self.pointers[&operation.get_operand_address(ctx)];
            output_effect.set_value_ref(input_effect);
            output_state.unwrap().set_value(input_effect.read(ptr));

            return;
        }

        // Fallback
        if let Some(output_state) = output_state {
            output_state.set_value(OptimisticScalar::default());
        }

        let mut new_output_effect = input_effect.clone();
        new_output_effect.arbitrary_write();
        output_effect.set_value(new_output_effect);
    }

    fn trans_terminator(
        &mut self,
        operation: Ptr<Operation>,
        input_effect: &Self::Effect,
        input_states: &[&Self::Value],
        output_effects: &mut [&mut DataflowSlot<Self::Effect>],
    ) {
        let ctx = self.ctx();

        // Conditional branch
        if let Some(_operation) = Operation::get_op::<CondBrOp>(operation, ctx) {
            let cond = input_states[0];
            let [truthy, falsy] = output_effects else {
                unreachable!()
            };

            // TODO

            return;
        }

        // Unconditional branch
        if let Some(_operation) = Operation::get_op::<BrOp>(operation, ctx) {
            for output_effect in output_effects {
                output_effect.set_value_ref(input_effect);
            }

            return;
        }

        // Fallback
        let mut new_output_effect = input_effect.clone();
        new_output_effect.arbitrary_write();

        for output_effect in output_effects {
            output_effect.set_value_ref(&new_output_effect);
        }
    }
}

// === Lattices === //

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PtrIdx(pub u32);

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PointeeEffectLattice {
    pub no_alias: NoAliasSet,
    pub known_pointees: KnownPointeeMap,
}

impl PointeeEffectLattice {
    pub fn read(&self, ptr: PtrIdx) -> OptimisticScalar {
        match self.known_pointees.raw.get(&ptr) {
            Some(&value) => OptimisticScalar::Known(value),
            None => OptimisticScalar::Unknown(Some(LoadHypothesis {
                read_src: ptr,
                branches: self.known_pointees.clone(),
            })),
        }
    }

    pub fn write(&mut self, ptr: PtrIdx, value: &OptimisticScalar) {
        let value = match value {
            OptimisticScalar::Known(value) => Some(*value),
            OptimisticScalar::Unknown(_) => None,
        };

        let map = Rc::make_mut(&mut self.known_pointees.raw);

        map.retain(|&other_ptr, &mut other_value| {
            if self.no_alias.has(ptr, other_ptr) {
                return true;
            }

            Some(other_value) == value
        });

        if let Some(value) = value {
            map.insert(ptr, value);
        }
    }

    pub fn arbitrary_write(&mut self) {
        self.known_pointees.clear();
    }

    pub fn join(&mut self, other: &Self) {
        self.no_alias.join(&other.no_alias);
        self.known_pointees.join(&other.known_pointees);
    }
}

#[derive(Clone, Eq, PartialEq, Default)]
pub struct NoAliasSet {
    pairs: FxHashSet<[PtrIdx; 2]>,
}

impl fmt::Debug for NoAliasSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pairs.fmt(f)
    }
}

impl NoAliasSet {
    pub fn add(&mut self, lhs: PtrIdx, rhs: PtrIdx) {
        let mut pair = [lhs, rhs];
        pair.sort();
        self.pairs.insert(pair);
    }

    pub fn has(&self, lhs: PtrIdx, rhs: PtrIdx) -> bool {
        let mut pair = [lhs, rhs];
        pair.sort();
        self.pairs.contains(&pair)
    }

    pub fn join(&mut self, other: &Self) {
        self.pairs.retain(|pair| other.pairs.contains(pair));
    }
}

#[derive(Clone, Eq, PartialEq, Default)]
pub struct KnownPointeeMap {
    pub raw: Rc<FxHashMap<PtrIdx, u64>>,
}

impl fmt::Debug for KnownPointeeMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.raw.fmt(f)
    }
}

impl KnownPointeeMap {
    pub fn clear(&mut self) {
        if let Some(map) = Rc::get_mut(&mut self.raw) {
            map.clear();
        } else {
            self.raw = Rc::new(FxHashMap::default());
        }
    }

    pub fn join(&mut self, other: &Self) {
        Rc::make_mut(&mut self.raw)
            .retain(|key, value| other.raw.get(key).is_none_or(|other| value == other));
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OptimisticScalar {
    Known(u64),
    Unknown(Option<LoadHypothesis>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LoadHypothesis {
    pub read_src: PtrIdx,
    pub branches: KnownPointeeMap,
}

impl Default for OptimisticScalar {
    fn default() -> Self {
        Self::Unknown(None)
    }
}

impl OptimisticScalar {
    pub fn map(&mut self, mut f: impl FnMut(u64) -> u64) {
        match self {
            OptimisticScalar::Known(value) => {
                *value = f(*value);
            }
            OptimisticScalar::Unknown(Some(load_hypothesis)) => {
                for value in Rc::make_mut(&mut load_hypothesis.branches.raw).values_mut() {
                    *value = f(*value);
                }
            }
            OptimisticScalar::Unknown(None) => {
                // (nothing to update)
            }
        }
    }

    pub fn join(&mut self, other: &OptimisticScalar) {
        match (&mut *self, other) {
            (OptimisticScalar::Known(lhs), OptimisticScalar::Known(rhs)) if lhs == rhs => {
                // (no-op)
            }
            (OptimisticScalar::Unknown(Some(lhs)), OptimisticScalar::Unknown(Some(rhs)))
                if lhs.read_src == rhs.read_src =>
            {
                lhs.branches.join(&rhs.branches);
            }
            _ => {
                *self = OptimisticScalar::default();
            }
        }
    }
}
