#![allow(deprecated)]

use std::env;

use anyhow::Context as _;
use pliron::{
    arg_error_noloc,
    builtin::op_interfaces::SingleBlockRegionInterface as _,
    context::Context,
    linked_list::{ContainsLinkedList, LinkedList},
    op::verify_op,
    pass::{AnalysisManager, Pass, Passes},
    printable::Printable as _,
};
use pliron_llvm::{
    from_llvm_ir,
    llvm_sys::core::{LLVMContext, LLVMModule},
    to_llvm_ir::{self, ToLLVMOptions},
};

use crate::{
    analysis::PointeeConstantsFacts,
    annotate::MeowAnnotate,
    dataflow::{DataflowGraph, dataflow_pretty},
};

pub mod analysis;
pub mod annotate;
pub mod dataflow;
pub mod lattice;

fn main() -> anyhow::Result<()> {
    // Parse arguments
    let args = env::args().collect::<Vec<_>>();
    let args = args.iter().map(|v| v.as_str()).collect::<Vec<_>>();
    let args = &args[..];

    let [_bin_name, input_path, output_path] = *args else {
        anyhow::bail!("invalid usage");
    };

    // Import LLVM bytecode as pliron-llvm
    let mut ctx = Context::new();
    let llvm_ctx = LLVMContext::default();

    let module = LLVMModule::from_ir_in_file(&llvm_ctx, input_path)
        .map_err(|err| arg_error_noloc!("{}", err))?;

    let module = from_llvm_ir::convert_module(&mut ctx, &module)?;

    verify_op(&module, &mut ctx)
        .with_context(|| format!("verification failed\n{}", module.disp(&mut ctx)))?;

    let mut analysis_mgr = AnalysisManager::default();

    let module_defs = module.get_body(&ctx, 0);

    // Run analyses
    for op in module_defs.deref(&ctx).iter(&ctx) {
        analysis_mgr.compute_analysis::<PointeeConstantsFacts>(op, &ctx)?;

        let PointeeConstantsFacts {
            facts: Some(scratch),
        } = &*analysis_mgr.try_get_analysis(op).unwrap()
        else {
            continue;
        };

        let graph = &*analysis_mgr.try_get_analysis::<DataflowGraph>(op).unwrap();

        println!("{}", dataflow_pretty(&ctx, graph, scratch));
    }

    // Run passes
    let mut passes = Passes::default();
    passes.add_pass(MeowAnnotate);

    {
        let mut curr = module_defs.deref(&ctx).get_head();

        while let Some(op) = curr {
            curr = op.deref(&ctx).get_next();

            passes.run(op, &mut ctx, &mut analysis_mgr).unwrap();
        }
    }

    // Write back output
    to_llvm_ir::convert_module(&ctx, &llvm_ctx, module, ToLLVMOptions::default())?
        .asm_to_file(output_path)
        .map_err(|err| anyhow::anyhow!("failed to write out assembly: {err}"))?;

    Ok(())
}
