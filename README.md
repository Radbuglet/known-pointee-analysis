# Assignment for Oct. 19

Riley Denoyer

The MRP for the bug is in `res/input.cpp`, the facts derived for the program are in `res/output.txt`, and `res/output.ll` is instrumented to check that those facts hold at runtime. You can build and run the instrumented binary with `./run.sh`

My analysis can conclude...

- What each local pointer's pointee value should equal.
- Which sets of pointers do not alias.
- The resulting value of a load of a pointer whose pointee is known.
- Whether code is dead.

I only managed to instrument the program to check the last two facts.

`input.cpp` exercises I bug I was aware of when I first wrote my analysis. In particular, my analysis does a bad job of defining what it means for two pointers not to alias. The transfer function for `llvm.store` assumes that, if two pointers do not alias, writes to one pointer will have zero impact on the latter. In other words, it interprets non-aliasing as meaning that the two pointers' underlying bytes have no overlap. The transfer function for `llvm.cond_br`, meanwhile, can only show that two pointers of the same size do not share the same base address. This means that we can use the fact that two pointers do not share the same base address to conclude incorrect facts about the effects of stores.
