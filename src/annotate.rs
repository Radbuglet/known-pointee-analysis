use pliron::{
    context::{Context, Ptr},
    operation::Operation,
    pass::{AnalysisManager, Pass, PassResult},
    result::Error as PlironError,
};

pub struct MeowAnnotate;

impl Pass for MeowAnnotate {
    fn name(&self) -> &str {
        "meow"
    }

    fn run(
        &mut self,
        op: Ptr<Operation>,
        ctx: &mut Context,
        analyses: &mut AnalysisManager,
    ) -> Result<PassResult, PlironError> {
        todo!()
    }
}
