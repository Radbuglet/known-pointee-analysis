use std::ops::BitOr;

use index_vec::{IndexVec, define_index_type};
use pliron::{
    builtin::op_interfaces::AtMostOneRegionInterface as _,
    context::{Context, Ptr},
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    result::Error as PlironError,
};
use pliron_llvm::ops::FuncOp;
use rustc_hash::FxHashSet;

use crate::dataflow::{DataflowAnalysis, DataflowGraph, DataflowScratch, DataflowSlot, DirtyFlag};

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

        let mut analysis = MeowAnalysis { ctx, graph: &graph };
        let mut scratch = DataflowScratch::new();

        analysis.run(&mut scratch);

        Ok(Self {})
    }
}

// === Analysis === //

pub struct MeowAnalysis<'a> {
    ctx: &'a Context,
    graph: &'a DataflowGraph,
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
        KnownPointees::default()
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
        todo!()
    }

    fn trans_value_phi(
        &mut self,
        input_states: &[&Self::Value],
        output_state: &mut DataflowSlot<Self::Value>,
    ) {
        todo!()
    }

    fn trans_stmt(
        &mut self,
        operation: Ptr<Operation>,
        input_effect: &Self::Effect,
        output_effect: &mut DataflowSlot<Self::Effect>,
        input_states: &[&Self::Value],
        output_state: Option<&mut DataflowSlot<Self::Value>>,
    ) {
        todo!()
    }

    fn trans_terminator(
        &mut self,
        operation: Ptr<Operation>,
        input_effect: &Self::Effect,
        input_states: &[&Self::Value],
        output_effects: &[&mut DataflowSlot<Self::Effect>],
    ) {
        todo!()
    }
}

// === Lattices === //

define_index_type! {
    pub struct PtrIdx = u32;
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct KnownPointees {
    no_alias: NoAliasSet,
    pointees: IndexVec<PtrIdx, KnownScalar>,
}

impl KnownPointees {
    pub fn read(&self, ptr: PtrIdx) -> KnownScalar {
        self.pointees[ptr]
    }

    pub fn write(&mut self, ptr: PtrIdx, value: KnownScalar, flag: &mut DirtyFlag) {
        for (other, scalar) in self.pointees.iter_mut_enumerated() {
            if !self.no_alias.has(ptr, other) {
                scalar.join(value, flag);
            }
        }
    }

    pub fn join(&mut self, other: &Self, flag: &mut DirtyFlag) {
        self.no_alias.join(&other.no_alias, flag);

        for (lhs, rhs) in self.pointees.iter_mut().zip(&other.pointees) {
            lhs.join(*rhs, flag);
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

    pub fn join(&mut self, other: &Self, flag: &mut DirtyFlag) {
        self.pairs.retain(|pair| {
            let is_contained = other.pairs.contains(pair);
            *flag |= DirtyFlag::from_is_dirty(!is_contained);
            is_contained
        });
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum KnownScalar {
    Bottom,
    Exactly(u64),
    Top,
}

impl KnownScalar {
    pub fn join(&mut self, other: Self, flag: &mut DirtyFlag) {
        let res = *self | other;
        *flag |= DirtyFlag::from_is_dirty(*self != res);
        *self = res;
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
