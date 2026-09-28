use pliron::{
    builtin::op_interfaces::AtMostOneRegionInterface as _,
    context::{Context, Ptr},
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    result::Error as PlironError,
};
use pliron_llvm::ops::FuncOp;

use crate::dataflow::DataflowGraph;

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

        let graph = analyses
            .compute_analysis::<DataflowGraph>(raw_op, ctx)
            .unwrap();

        Ok(Self {})
    }
}
