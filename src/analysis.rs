use pliron::{
    builtin::op_interfaces::AtMostOneRegionInterface,
    context::{Context, Ptr},
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    result::Error as PlironError,
};
use pliron_llvm::ops::FuncOp;

pub struct PointeeConstantsFacts {}

impl Analysis for PointeeConstantsFacts {
    fn name(&self) -> &str {
        "pointee constants"
    }

    fn compute(
        op: Ptr<Operation>,
        ctx: &Context,
        _analyses: &mut AnalysisManager,
    ) -> Result<Self, PlironError> {
        let Some(op) = Operation::get_op::<FuncOp>(op, ctx) else {
            return Ok(Self {});
        };

        let Some(body) = op.get_region(ctx) else {
            return Ok(Self {});
        };

        Ok(Self {})
    }
}
