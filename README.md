# Known Pointee Analysis

Weeks 5–6 assignment for CS 6475.

To run this analysis, execute `./run.sh`. This will compile the program in `input.cpp` and produce an annotated control-flow graph in `output.txt`. If you have LLVM bytecode available, you can run `cargo run -- path_to_input.ll`.

The existing `output.txt` demonstrates a deduction of `foo` and `bar` not aliasing and, if the call to `arbitrary_side_effect` is commented out, the detection of dead-code that LLVM didn't eliminate.

This analysis works by determining...

- The known scalar value a given SSA term has.
- The known scalar values that the direct pointee of every SSA pointer term has at every statement location.
- The set of SSA pointer terms that cannot possibly alias at every statement location.
- Whether the program is dead at a given statement location since it's very easy to track and can help with some analyses.
- If a scalar value is not known for a given SSA term but that value was obtained by an `llvm.load` operation, it tracks what that SSA term could equal given all possible aliasings of the loaded pointer with other pointers with known values.
- We can use `llvm.cond_br` and these speculative alias guesses to determine which pointers are allowed to alias depending on the branch taken at runtime.

I'm not sure whether our analysis is sound if pointers are allowed to partially alias but I haven't found a counterexample for that yet. Luckily, I think this can be taken for granted so long as we only consider `llvm.load` since, afaict, it requires pointers to be aligned to their word size but maybe I have to track the load size and required alignment of the pointer to make assumption properly.
