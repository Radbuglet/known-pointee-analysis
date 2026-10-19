use std::num::NonZeroUsize;

use pliron::{
    builtin::{
        attributes::IntegerAttr,
        op_interfaces::{CallOpCallable, OneResultInterface},
        types::{IntegerType, Signedness},
    },
    context::{Context, Ptr},
    ident,
    irbuild::{
        inserter::{IRInserter, Inserter, OpInsertionPoint},
        listener::DummyListener,
    },
    linked_list::{ContainsLinkedList, LinkedList},
    op::Op,
    operation::Operation,
    pass::{AnalysisManager, Pass, PassResult},
    result::Error as PlironError,
    utils::apint::APInt,
};
use pliron_llvm::{
    ops::{CallOp, ConstantOp},
    types::{FuncType, VoidType},
};

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

        let mut loc_counter = 0u32;

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
                        let bool_ty = IntegerType::get(ctx, 1, Signedness::Signless);
                        let uint_ty = IntegerType::get(ctx, 32, Signedness::Unsigned);

                        let false_op = ConstantOp::new(
                            ctx,
                            Box::new(IntegerAttr::new(
                                bool_ty,
                                APInt::from_u32(0, NonZeroUsize::new(1).unwrap()),
                            )),
                        );

                        let site_op = ConstantOp::new(
                            ctx,
                            Box::new(IntegerAttr::new(
                                uint_ty,
                                APInt::from_u32(loc_counter, NonZeroUsize::new(32).unwrap()),
                            )),
                        );

                        loc_counter += 1;

                        let assert_op = CallOp::new(
                            ctx,
                            CallOpCallable::Direct(ident!("meow_assert")),
                            FuncType::get(
                                ctx,
                                VoidType::get(ctx).to_handle(),
                                [bool_ty.to_handle(), uint_ty.to_handle()].into(),
                                false,
                            ),
                            [false_op.get_result(ctx), site_op.get_result(ctx)].into(),
                        );

                        inserter.insert_operation(ctx, false_op.get_operation());
                        inserter.insert_operation(ctx, site_op.get_operation());
                        inserter.insert_operation(ctx, assert_op.get_operation());
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
