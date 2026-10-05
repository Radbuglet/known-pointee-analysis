use pliron::{
    attribute::Attribute,
    builtin::{
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
    attributes::ICmpPredicateAttr,
    op_interfaces::VolatilityOpInterface,
    ops::{BrOp, CondBrOp, ConstantOp, FuncOp, ICmpOp, LoadOp, StoreOp, TruncOp},
    types::PointerType,
};
use rustc_hash::FxHashMap;

use crate::{
    dataflow::{DataflowAnalysis, DataflowGraph, DataflowScratch, DataflowSlot},
    lattice::{EffectLattice, PtrIdx, ValueLattice},
};

// === Driver === //

pub struct PointeeConstantsFacts {
    pub facts: Option<DataflowScratch<EffectLattice, ValueLattice>>,
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
            let next_idx = PtrIdx(self.pointers.len() as u32);
            self.pointers.entry(value).or_insert(next_idx);
        }
    }
}

impl<'a> DataflowAnalysis<'a> for MeowAnalysis<'a> {
    type Effect = EffectLattice;
    type Value = ValueLattice;

    fn ctx(&self) -> &'a Context {
        self.ctx
    }

    fn graph(&self) -> &'a DataflowGraph {
        self.graph
    }

    fn init_effect(&mut self, _is_input: bool) -> Self::Effect {
        EffectLattice::default()
    }

    fn init_value(&mut self, _is_input: bool) -> Self::Value {
        ValueLattice::default()
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
            target.join_monotonic(other);
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
            target.join_monotonic(other);
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

        if matches!(input_effect, EffectLattice::Dead) {
            if let Some(output_state) = output_state {
                output_state.set_value(ValueLattice::Dead);
            }

            output_effect.set_value(EffectLattice::Dead);
            return;
        }

        // Constant
        if let Some(operation) = Operation::get_op::<ConstantOp>(operation, ctx)
            && let Some(constant) =
                (&*operation.get_value(ctx) as &dyn Attribute).downcast_ref::<IntegerAttr>()
        {
            output_effect.set_value_ref(input_effect);

            output_state
                .unwrap()
                .set_value(ValueLattice::KnownConst(constant.value().to_u64()));

            return;
        }

        // Truncate
        if let Some(operation) = Operation::get_op::<TruncOp>(operation, ctx)
            && let result_ty = operation.result_type(ctx)
            && let Some(int_ty) = result_ty.deref(ctx).downcast_ref::<IntegerType>()
        {
            output_effect.set_value_ref(input_effect);

            let mut new_output_state = input_states[0].clone();
            new_output_state.map_monotonic(|value| value & ((1u64 << int_ty.width()) - 1));
            output_state.unwrap().set_value(new_output_state);

            return;
        }

        // ICmp
        if let Some(operation) = Operation::get_op::<ICmpOp>(operation, ctx)
            && operation.predicate(ctx) == ICmpPredicateAttr::EQ
        {
            output_effect.set_value_ref(input_effect);

            let [lhs, rhs] = input_states else {
                unreachable!()
            };

            output_state
                .unwrap()
                .set_value(ValueLattice::map_pair_monotonic(lhs, rhs, |lhs, rhs| {
                    if lhs == rhs { 1 } else { 0 }
                }));

            return;
        }

        // Store
        if let Some(operation) = Operation::get_op::<StoreOp>(operation, ctx)
            && !operation.is_volatile(ctx)
        {
            let ptr = self.pointers[&operation.get_operand_address(ctx)];
            let mut new_output_effect = input_effect.clone();
            new_output_effect.write_monotonic(ptr, input_states[0]);
            output_effect.set_value(new_output_effect);

            assert!(output_state.is_none());

            return;
        }

        // Load
        if let Some(operation) = Operation::get_op::<LoadOp>(operation, ctx)
            && !operation.is_volatile(ctx)
        {
            let ptr = self.pointers[&operation.get_operand_address(ctx)];
            output_effect.set_value_ref(input_effect);
            output_state
                .unwrap()
                .set_value(input_effect.read_monotonic(ptr));

            return;
        }

        // Fallback
        if let Some(output_state) = output_state {
            output_state.set_value(ValueLattice::Unknown);
        }

        let mut new_output_effect = input_effect.clone();
        new_output_effect.arbitrary_write_monotonic();
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

        if matches!(input_effect, EffectLattice::Dead) {
            for output_effect in output_effects {
                output_effect.set_value_ref(input_effect);
            }

            return;
        }

        // Conditional branch
        if let Some(_operation) = Operation::get_op::<CondBrOp>(operation, ctx) {
            let cond = input_states[0];
            let [truthy, falsy] = output_effects else {
                unreachable!()
            };

            match cond.promote_load_non_monotonic() {
                ValueLattice::Dead => {
                    for output_effect in output_effects {
                        output_effect.set_value(EffectLattice::Dead);
                    }
                }
                ValueLattice::KnownLoad(hypothesis) => {
                    for (output, taken_if) in [(truthy, 1), (falsy, 0)] {
                        let mut new_output = input_effect.clone();

                        for (&if_aliased_with, &we_get) in hypothesis.branches.raw.iter() {
                            if taken_if != we_get {
                                // If this branch is taken *iff* the scrutinee is `taken_if` and we
                                // know that the scrutinee would be the opposite—`we_get`—if, at the
                                // point `hypothesis.read_src` was last loaded from memory, it
                                // aliased with `if_aliased_with`, we know that, for the branch to
                                // be taken, `hypothesis.read_src` and `if_aliased_with` may not
                                // alias.
                                new_output
                                    .push_no_alias_monotonic(hypothesis.read_src, if_aliased_with);
                            }
                        }

                        output.set_value(new_output);
                    }
                }
                ValueLattice::KnownConst(value) => {
                    for (output, taken_if) in [(truthy, 1), (falsy, 0)] {
                        if value == taken_if {
                            // Alive
                            output.set_value_ref(input_effect);
                        } else {
                            // Dead
                            output.set_value(EffectLattice::Dead);
                        }
                    }
                }
                ValueLattice::Unknown => {
                    for output_effect in output_effects {
                        output_effect.set_value_ref(input_effect);
                    }
                }
            }

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
        new_output_effect.arbitrary_write_monotonic();

        for output_effect in output_effects {
            output_effect.set_value_ref(&new_output_effect);
        }
    }
}
