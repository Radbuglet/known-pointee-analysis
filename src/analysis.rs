use std::ops::{BitOr, BitOrAssign};

use index_vec::{IndexVec, define_index_type};
use pliron::{
    builtin::op_interfaces::AtMostOneRegionInterface as _,
    context::{Context, Ptr},
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    printable::Printable,
    result::Error as PlironError,
    value::Value,
};
use pliron_llvm::ops::{ConstantOp, FuncOp, LoadOp, StoreOp};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::dataflow::{DataflowAnalysis, DataflowGraph, DataflowScratch, DataflowSlot};

// === Driver === //

pub struct PointeeConstantsFacts {}

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
            return Ok(Self {});
        };

        if op.get_region(ctx).is_none() {
            return Ok(Self {});
        }

        let graph = analyses.get_analysis::<DataflowGraph>(raw_op, ctx).unwrap();

        let mut analysis = MeowAnalysis {
            ctx,
            graph: &graph,
            pointers: FxHashMap::default(),
        };
        let mut scratch = DataflowScratch::new();

        analysis.run(&mut scratch);

        Ok(Self {})
    }
}

// === Analysis === //

pub struct MeowAnalysis<'a> {
    ctx: &'a Context,
    graph: &'a DataflowGraph,
    pointers: FxHashMap<Value, PtrIdx>,
}

impl<'a> DataflowAnalysis<'a> for MeowAnalysis<'a> {
    type Effect = KnownPointees;
    type Value = KnownScalar;

    fn ctx(&self) -> &'a Context {
        self.ctx
    }

    fn graph(&self) -> &'a DataflowGraph {
        self.graph
    }

    fn init_effect(&mut self, _is_input: bool) -> Self::Effect {
        KnownPointees::new(self.pointers.len())
    }

    fn init_value(&mut self, is_input: bool) -> Self::Value {
        if is_input {
            KnownScalar::Top
        } else {
            KnownScalar::Bottom
        }
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
        let Some((&&first, remainder)) = input_states.split_first() else {
            return;
        };

        let mut target = first;

        for &&other in remainder {
            target |= other;
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
        eprintln!("{}", operation.disp(ctx));

        /*
        // Constant
        if let Some(operation) = Operation::get_op::<ConstantOp>(operation, ctx) {
            println!("{}", operation.get_value(ctx).disp(ctx));

            // output_effect.set_value_ref(input_effect);
            // output_state
            //     .unwrap()
            //     .set_value(KnownScalar::Exactly(operation.get_value(ctx)));

            todo!();

            return;
        }

        // Store
        if let Some(operation) = Operation::get_op::<StoreOp>(operation, ctx) {
            let ptr = self.pointers[&operation.get_operand_address(ctx)];
            let mut new_output_effect = input_effect.clone();
            new_output_effect.write(ptr, *input_states[1]);
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
        */

        // Fallback
        if let Some(output_state) = output_state {
            output_state.set_value(KnownScalar::Top);
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
        eprintln!("{}", operation.disp(ctx));

        // Conditional branch
        // TODO

        // Fallback
        let mut new_output_effect = input_effect.clone();
        new_output_effect.arbitrary_write();

        for output_effect in output_effects {
            output_effect.set_value_ref(&new_output_effect);
        }
    }
}

// === Lattices === //

define_index_type! {
    pub struct PtrIdx = u32;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KnownPointees {
    no_alias: NoAliasSet,
    pointees: IndexVec<PtrIdx, KnownScalar>,
}

impl KnownPointees {
    pub fn new(index_count: usize) -> Self {
        Self {
            no_alias: NoAliasSet::default(),
            pointees: IndexVec::from_iter((0..index_count).map(|_| KnownScalar::Bottom)),
        }
    }

    pub fn read(&self, ptr: PtrIdx) -> KnownScalar {
        self.pointees[ptr]
    }

    pub fn write(&mut self, ptr: PtrIdx, value: KnownScalar) {
        for (other, scalar) in self.pointees.iter_mut_enumerated() {
            if !self.no_alias.has(ptr, other) {
                *scalar = *scalar | value;
            }
        }
    }

    pub fn arbitrary_write(&mut self) {
        for ptr in &mut self.pointees {
            *ptr = KnownScalar::Top;
        }
    }

    pub fn join(&mut self, other: &Self) {
        self.no_alias.join(&other.no_alias);

        for (lhs, rhs) in self.pointees.iter_mut().zip(&other.pointees) {
            *lhs |= *rhs;
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct NoAliasSet {
    pairs: FxHashSet<[PtrIdx; 2]>,
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

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum KnownScalar {
    Bottom,
    Exactly(u64),
    Top,
}

impl BitOrAssign for KnownScalar {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitOr for KnownScalar {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        use KnownScalar::*;

        match (self, rhs) {
            (Bottom, Bottom) => Bottom,
            (Bottom, Exactly(value)) | (Exactly(value), Bottom) => Exactly(value),
            (Exactly(lhs), Exactly(rhs)) => {
                if lhs == rhs {
                    Exactly(lhs)
                } else {
                    Top
                }
            }
            (Top, _) | (_, Top) => Top,
        }
    }
}
