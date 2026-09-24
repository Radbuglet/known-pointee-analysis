use pliron::{
    builtin::op_interfaces::BranchOpInterface,
    context::{Context, Ptr},
    linked_list::ContainsLinkedList,
    op::op_impls,
    operation::Operation,
    printable::Printable,
    region::Region,
};
use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use smallvec::SmallVec;

pub trait DataflowAnalysis<'c>: Sized {
    type Effects: Lattice<'c, Self>;
    type Var: Lattice<'c, Self>;

    fn ctx(&self) -> &'c Context;

    fn analyze(&mut self, region: Ptr<Region>) {
        let ctx = self.ctx();

        new_key_type! {
            struct NodeIdx;
        }

        struct Node<TEffect, TVar> {
            input_state: TEffect,
            operands: Vec<NodeIdx>,
            successors: SmallVec<[NodeIdx; 1]>,
            defined_var: Option<NodeVar<TVar>>,
            is_queued: bool,
        }

        struct NodeVar<TVar> {
            state: TVar,
            consumers: Vec<NodeIdx>,
        }

        // Import the operations into an operation graph
        let mut nodes = SlotMap::<NodeIdx, Node<Self::Effects, Self::Var>>::default();
        let mut mapping = FxHashMap::<Ptr<Operation>, NodeIdx>::default();

        for bb in region.deref(ctx).iter(ctx) {
            let terminator = bb
                .deref(ctx)
                .get_terminator(ctx)
                .expect("basic blocks must have a terminator");

            for op in bb.deref(ctx).iter(ctx) {
                // Create placeholder
                let node = nodes.insert(Node {
                    input_state: Self::Effects::fresh_bot(self),
                    operands: Vec::new(),        // (late init)
                    successors: SmallVec::new(), // (late init)
                    defined_var: None,           // (late init)
                    is_queued: false,
                });

                mapping.insert(op, node);

                // Validate
                assert_eq!(op.deref(ctx).num_regions(), 0);

                if op == terminator {
                    match op.deref(ctx).get_num_successors() {
                        0 => {
                            // (no need to implement `BranchOpInterface`)
                        }
                        1.. => {
                            assert!(
                                op_impls::<dyn BranchOpInterface>(
                                    Operation::get_op_dyn(op, ctx).op_ref()
                                ),
                                "not a branch:\n{}",
                                op.disp(ctx)
                            );
                        }
                    }

                    assert_eq!(op.deref(ctx).get_num_results(), 0);
                } else {
                    assert_eq!(
                        op.deref(ctx).get_num_successors(),
                        0,
                        "statements cannot have non-trivial successors"
                    );

                    assert!(
                        matches!(op.deref(ctx).get_num_results(), 0..=1),
                        "statements can have at most one result"
                    );
                }
            }
        }

        for bb in region.deref(ctx).iter(ctx) {
            let terminator = bb.deref(ctx).get_terminator(ctx);

            for op in bb.deref(ctx).iter(ctx) {
                let node = mapping[&op];

                // Initialize operands
                // TODO

                // Initialize successors
                // TODO

                // Initialize reverse dependencies
                // TODO
            }
        }

        // Run dataflow
        // TODO
    }
}

pub trait Lattice<'c, D: DataflowAnalysis<'c>> {
    fn fresh_top(df: &mut D) -> Self;

    fn fresh_bot(df: &mut D) -> Self;

    fn join(df: &mut D, target: &mut Self, source: &Self) -> bool;
}

impl<'c, D: DataflowAnalysis<'c>> Lattice<'c, D> for () {
    fn fresh_top(_df: &mut D) -> Self {
        // (no-op)
    }

    fn fresh_bot(_df: &mut D) -> Self {
        // (no-op)
    }

    fn join(_df: &mut D, _target: &mut Self, _source: &Self) -> bool {
        false
    }
}
