use pliron::{
    builtin::op_interfaces::AtMostOneRegionInterface as _,
    context::{Context, Ptr},
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    result::Error as PlironError,
};
use pliron_llvm::ops::FuncOp;

use crate::dataflow::{DataflowAnalysis, DataflowOutput};

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

        PointeeConstantsDataflow { ctx }.analyze(body);

        Ok(Self {})
    }
}

pub struct PointeeConstantsDataflow<'c> {
    ctx: &'c Context,
}

impl<'c> DataflowAnalysis<'c> for PointeeConstantsDataflow<'c> {
    type ProgState = ();
    type VarState = ();

    fn ctx(&self) -> &'c Context {
        self.ctx
    }

    fn trans_statement(
        &mut self,
        input_prog: &Self::ProgState,
        input_vars: &[&Self::VarState],
        output_prog: DataflowOutput<'_, Self::ProgState>,
        output_var: Option<DataflowOutput<'_, Self::VarState>>,
    ) {
        todo!()
    }

    fn trans_terminator(&mut self) {
        todo!()
    }
}
