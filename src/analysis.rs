use pliron::{
    builtin::op_interfaces::{AtMostOneRegionInterface as _, BranchOpInterface},
    context::{Context, Ptr},
    linked_list::ContainsLinkedList as _,
    op::op_cast,
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

        for bb in body.deref(ctx).iter(ctx) {
            if let Some(term) = bb.deref(ctx).get_terminator(ctx)
                && let term = Operation::get_op_dyn(term, ctx)
                && let Some(term) = op_cast::<dyn BranchOpInterface>(term.op_ref())
            {
                println!("{}", term.disp(ctx));
            }
        }

        Ok(Self {})
    }
}
