use pliron::{
    builtin::op_interfaces::AtMostOneRegionInterface as _,
    context::{Context, Ptr},
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    result::Error as PlironError,
    value::Value,
};
use pliron_llvm::ops::FuncOp;
use rustc_hash::FxHashSet;

use crate::dataflow::{DataflowAnalysis, DataflowGraph, DataflowScratch, DataflowSlot};

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

        let mut analysis = MyAnalysis { ctx, graph: &graph };
        let mut scratch = DataflowScratch::new();

        analysis.run(&mut scratch);

        Ok(Self {})
    }
}

struct MyAnalysis<'a> {
    ctx: &'a Context,
    graph: &'a DataflowGraph,
}

#[derive(Default)]
struct Effects {
    non_aliasing_pairs: FxHashSet<[Value; 2]>,
}

#[derive(Debug, Copy, Clone)]
enum KnownPointee {
    Bottom,
    Top,
    Exactly(u64),
}

impl<'a> DataflowAnalysis<'a> for MyAnalysis<'a> {
    type Effect = Effects;
    type Value = KnownPointee;

    fn ctx(&self) -> &'a Context {
        self.ctx
    }

    fn graph(&self) -> &'a DataflowGraph {
        self.graph
    }

    fn init_effect(&mut self, _is_input: bool) -> Self::Effect {
        Effects::default()
    }

    fn init_value(&mut self, is_input: bool) -> Self::Value {
        if is_input {
            KnownPointee::Top
        } else {
            KnownPointee::Bottom
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
