use pliron::{
    context::{Context, Ptr},
    irbuild::{
        inserter::{IRInserter, Inserter, OpInsertionPoint},
        listener::DummyListener,
    },
    linked_list::{ContainsLinkedList, LinkedList},
    op::Op,
    operation::Operation,
    pass::{AnalysisManager, Pass, PassResult},
    result::Error as PlironError,
};
use pliron_llvm::ops::UnreachableOp;

use crate::{analysis::PointeeConstantsFacts, dataflow::DataflowGraph, lattice::EffectLattice};

pub struct MeowAnnotate;

impl Pass for MeowAnnotate {
    fn name(&self) -> &str {
        "meow"
    }

    fn run(
        &mut self,
        op: Ptr<Operation>,
        ctx: &mut Context,
        analysis_mgr: &mut AnalysisManager,
    ) -> Result<PassResult, PlironError> {
        analysis_mgr.compute_analysis::<PointeeConstantsFacts>(op, &ctx)?;

        let PointeeConstantsFacts {
            facts: Some(scratch),
        } = &*analysis_mgr.try_get_analysis(op).unwrap()
        else {
            return Ok(PassResult::default());
        };

        let graph = &*analysis_mgr.try_get_analysis::<DataflowGraph>(op).unwrap();
        let region = op.deref(ctx).get_region(0);

        let mut bb_iter = region.deref(ctx).get_head();

        while let Some(bb) = bb_iter {
            bb_iter = bb.deref(ctx).get_next();

            let mut op_iter = bb.deref(ctx).get_head();

            while let Some(op) = op_iter {
                op_iter = op.deref(ctx).get_next();

                let input_effect =
                    scratch.effect(graph.node_defs[graph.op_map[&op]].unwrap_input_effect());

                let mut inserter =
                    IRInserter::<DummyListener>::new(OpInsertionPoint::BeforeOperation(op));

                match input_effect {
                    EffectLattice::Dead => {
                        let unreachable = UnreachableOp::new(ctx).get_operation();
                        inserter.insert_operation(ctx, unreachable);
                    }
                    EffectLattice::Alive {
                        no_alias,
                        known_pointees,
                    } => {
                        // TODO
                    }
                }
            }
        }

        Ok(PassResult::default())
    }
}
