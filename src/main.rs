use std::env;

use anyhow::Context as _;
use pliron::{
    arg_error_noloc, builtin::op_interfaces::SingleBlockRegionInterface as _, context::Context,
    linked_list::ContainsLinkedList, op::verify_op, pass::AnalysisManager,
    printable::Printable as _,
};
use pliron_llvm::{
    from_llvm_ir,
    llvm_sys::core::{LLVMContext, LLVMModule},
};

use crate::{
    analysis::PointeeConstantsFacts,
    dataflow::{DataflowGraph, dataflow_pretty},
    lattice::{EffectLattice, ValueLattice},
};

pub mod analysis;
pub mod dataflow;
pub mod lattice;

fn main() -> anyhow::Result<()> {
    // Parse arguments
    let args = env::args().collect::<Vec<_>>();
    let args = args.iter().map(|v| v.as_str()).collect::<Vec<_>>();
    let args = &args[..];

    let [_bin_name, input_path] = *args else {
        anyhow::bail!("invalid usage");
    };

    // Import LLVM bytecode as pliron-llvm
    // From: https://github.com/pliron-org/pliron/blob/500d7d1ac9fb346524d24b5f54b0172892e5ff87/pliron-llvm/llvm-opt/src/main.rs#L210
    let mut ctx = Context::new();
    let llvm_ctx = LLVMContext::default();

    let module = LLVMModule::from_ir_in_file(&llvm_ctx, input_path)
        .map_err(|err| arg_error_noloc!("{}", err))?;

    let module = from_llvm_ir::convert_module(&mut ctx, &module)?;

    verify_op(&module, &mut ctx)
        .with_context(|| format!("verification failed\n{}", module.disp(&mut ctx)))?;

    // Run an analysis
    // See: https://docs.rs/pliron/0.18.0/pliron/pass/index.html
    let mut analysis_mgr = AnalysisManager::default();

    // An `Operation`, in essence, is just...
    //
    // - a container for arbitrary metadata (e.g. literal arguments, but not SSA arguments or types,
    //   which are modelled elsewhere)
    //    - see `attributes`
    // - a set of interfaces which can be dynamically casted
    //    - these are defined statically per concrete operand type
    //    - see `concrete_op` and the `Op` trait
    //    - this also includes printing and parsing behavior
    // - a set of input and output SSA terms
    //    - each term has a type
    //    - see `operands` and `results`
    // - a set of successor blocks
    //    - see `successors`
    //    - arguments to these blocks are *operation defined*
    // - a set of child regions, containing a bunch of basic blocks which, in turn, contain more
    //   operations.
    //    - basic blocks use linked lists of operations, which is how operations know their position in
    //      basic blocks.
    // - a source location
    //    - see `loc`
    //
    // You define operations in dialects using the `#[pliron_op]` derive macro, which essentially
    // creates a new-type to provide type-safety around these operations.
    //
    // This abstraction can be used to model...
    //
    // - Modules
    // - Functions
    // - Statements
    // - Terminators
    //
    // `ModuleOp` models its child functions as a single child region with a single basic block
    // within that region which has no terminator. The operations within that "body" are module's
    // definitions. See the set of interfaces `ModuleOp` implements.
    //
    // This is honestly quite a clever modeling technique. Kudos!
    let module_defs = module.get_body(&ctx, 0);

    for op in module_defs.deref(&ctx).iter(&ctx) {
        analysis_mgr.compute_analysis::<PointeeConstantsFacts>(op, &ctx)?;

        let PointeeConstantsFacts {
            facts: Some(scratch),
        } = &*analysis_mgr.try_get_analysis(op).unwrap()
        else {
            continue;
        };

        let graph = &*analysis_mgr.try_get_analysis::<DataflowGraph>(op).unwrap();

        println!(
            "{}",
            dataflow_pretty(
                &ctx,
                graph,
                scratch,
                &|effect| match effect {
                    EffectLattice::Dead => false,
                    EffectLattice::Alive {
                        no_alias,
                        known_pointees,
                    } => {
                        !no_alias.pairs.is_empty() && !known_pointees.raw.is_empty()
                    }
                },
                &|value| match value {
                    ValueLattice::Dead => false,
                    ValueLattice::KnownConst(_) => false,
                    ValueLattice::KnownLoad(_) => false,
                    ValueLattice::Unknown => false,
                }
            )
        );
    }

    Ok(())
}
