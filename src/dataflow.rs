use index_vec::{IndexVec, define_index_type};
use pliron::{
    basic_block::BasicBlock,
    context::{Context, Ptr},
    operation::Operation,
    pass::{Analysis, AnalysisManager},
    region::Region,
    result::Error as PlironError,
};
use smallvec::SmallVec;

// === DataflowGraph === //

define_index_type! {
    struct DataflowNodeIdx = u32;
}

pub struct DataflowGraph {
    nodes: IndexVec<DataflowNodeIdx, DataflowNode>,
}

enum DataflowNode {
    /// A start-of-block node which stashes incoming effects and block variable states.
    StartOfBlock {
        block: Ptr<BasicBlock>,

        /// The statement to which this effect is forward.
        first_stmt: DataflowNodeIdx,

        /// Nodes which consume this block's variable states.
        output_consumers: SmallVec<[DataflowNodeIdx; 1]>,
    },
    /// A statement node which may produce a result and propagate that result elsewhere.
    Stmt {
        operation: Ptr<Operation>,

        /// `Stmt` and `StartOfBlock` nodes supplying our operand variable states.
        operands: SmallVec<[DataflowNodeIdxAndSlot; 2]>,

        /// `Stmt` and `StartOfBlock` nodes which consume our result.
        output_consumers: SmallVec<[DataflowNodeIdxAndSlot; 1]>,

        /// Subsequent node in this basic block.
        effect_successor: DataflowNodeIdx,
    },
    /// A terminator node which may propagate its effect to multiple different targets.
    Terminator {
        /// Where each successor `StartOfBlock` lives.
        effect_successors: SmallVec<[DataflowNodeIdx; 2]>,
    },
}

struct DataflowNodeIdxAndSlot {
    node: DataflowNodeIdx,
    output_idx: u32,
}

impl DataflowGraph {
    pub fn new(ctx: &Context, region: Ptr<Region>) -> Self {
        todo!()
    }
}

impl Analysis for DataflowGraph {
    fn name(&self) -> &str {
        "dataflow graph"
    }

    fn compute(
        op: Ptr<Operation>,
        ctx: &Context,
        _analyses: &mut AnalysisManager,
    ) -> Result<Self, PlironError>
    where
        Self: Sized,
    {
        todo!()
    }
}

// === DataflowAnalysis === //

// TODO
