use pliron::{
    builtin::op_interfaces::BranchOpInterface,
    context::{Context, Ptr},
    linked_list::ContainsLinkedList,
    op::op_impls,
    operation::Operation,
    printable::Printable,
    region::Region,
};

pub struct DataflowOutput<'a, T> {
    did_change: &'a mut bool,
    value: &'a mut T,
}

impl<'a, T> DataflowOutput<'a, T> {
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
        output_prog: DataflowOutput<'_, Self::ProgState>,
        output_var: Option<DataflowOutput<'_, Self::VarState>>,
    );

    fn trans_terminator(&mut self);

    fn analyze(&mut self, region: Ptr<Region>) {
        let ctx = self.ctx();

        // Validate basic blocks
        for bb in region.deref(ctx).iter(ctx) {
            let terminator = bb.deref(ctx).get_terminator(ctx);

            for op in bb.deref(ctx).iter(ctx) {
                assert_eq!(op.deref(ctx).num_regions(), 0);

                if Some(op) == terminator {
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
                    assert_eq!(op.deref(ctx).get_num_successors(), 0);
                    assert!(matches!(op.deref(ctx).get_num_results(), 0..=1));
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

impl<'c, D: DataflowAnalysis<'c>> Lattice<'c, D> for () {
    fn fresh_top(_df: &mut D) -> Self {
        // (no-op)
    }

    fn fresh_bot(_df: &mut D) -> Self {
        // (no-op)
    }

    fn join(_df: &mut D, _target: &mut Self, _sources: impl IntoIterator<Item = Self>) {
        // (no-op)
    }
}
