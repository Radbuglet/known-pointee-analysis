use pliron::{
    builtin::op_interfaces::BranchOpInterface,
    context::{Context, Ptr},
    linked_list::ContainsLinkedList,
    op::op_impls,
    operation::Operation,
    region::Region,
};

pub struct Output<'a, T> {
    did_change: &'a mut bool,
    value: &'a mut T,
}

impl<'a, T> Output<'a, T> {
    pub fn get(&self) -> &T {
        self.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.mark_dirty();
        self.value
    }

    pub fn get_mut_no_update(&mut self) -> &mut T {
        self.value
    }

    pub fn mark_dirty(&mut self) {
        *self.did_change = true;
    }
}

pub trait DataflowAnalysis<'c>: Sized {
    type ProgState: Lattice<'c, Self>;
    type VarState: Lattice<'c, Self>;

    fn ctx(&self) -> &'c Context;

    fn trans_statement(
        &mut self,
        input_prog: &Self::ProgState,
        input_vars: &[&Self::VarState],
        output_prog: Output<'_, Self::ProgState>,
        output_var: Output<'_, Self::VarState>,
    );

    fn trans_terminator(&mut self);

    fn analyze(&mut self, region: Ptr<Region>) {
        let ctx = self.ctx();

        // Validate regions
        for bb in region.deref(ctx).iter(ctx) {
            let terminator = bb.deref(ctx).get_terminator(ctx);

            for op in bb.deref(ctx).iter(ctx) {
                assert_eq!(op.deref(ctx).num_regions(), 0);

                if Some(op) == terminator {
                    assert!(op_impls::<dyn BranchOpInterface>(
                        Operation::get_op_dyn(op, ctx).op_ref()
                    ));
                    assert_eq!(op.deref(ctx).get_num_results(), 0);
                } else {
                    assert_eq!(op.deref(ctx).get_num_successors(), 0);
                    assert_eq!(op.deref(ctx).get_num_results(), 1);
                }
            }
        }

        // TODO: analyze
    }
}

pub trait Lattice<'c, D: DataflowAnalysis<'c>>: Clone {
    fn fresh_top(df: &mut D) -> Self;

    fn fresh_bot(df: &mut D) -> Self;

    fn join(df: &mut D, target: &mut Self, sources: impl IntoIterator<Item = Self>);
}
