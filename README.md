# Connection Tester

I was provided with a Rust program that emulates a really simple computer,
4 bytes of memory and 4 instructions. I was tasked to test it, no indications given.

While I cannot come up with multiple interesting test cases (maybe a few, but nothing to be able to confidently say "perfect"),
a machine can! Since the original program was simple enough to emulate, I decided to write a differential tester.
I wrote a small simulator and leveraged [`quickcheck`](https://github.com/BurntSushi/quickcheck)
to generate random commands to send.

## Running

Before you run the tester, you'll need to set up the remote along with the instructions provided
in challenge project.

You can run the tester with the following command:

```
cargo run --release
RUST_LOG=debug cargo run --release # for a more "complete" logging experience
```

## Errors

1. The original program doesn't attempt to handle overflow resulting in panics under debug mode
   and wraparounds in release mode.

   In the simulator I made use of `checked_sum` and `checked_mul` to detect this issue.

2. The original program doesn't handle indexes out of bounds, it simply crashes.

   In the simulator I used the "checked" version of index accessing — `get` — to detect this issue.
   I then "disabled" this case from the tester as it leads to not very interesting tests,
   where the program crashes fairly fast (in the test there's a 1 in 5 chance of hitting this case).


## Improvements

There are some improvements that could be made, in no particular order:

1. Manage the target binary by itself — The tester could run the original binary, so it could run
   and on crash or divergence, restart the binary automatically, instead of relying on a human to
   set the environment up (one example of this would be when running the target on release mode —
   the simulator will detect an overflow, but the target will not crash; if the tester runs again
   without resetting the target, the states will be out of sync).

2. More extensive fuzzing — the original program only reads 3 bytes from the stream at a time,
   testing commands both bigger and smaller than expected might yield interesting results
   as well as invalid commands.

3. Better errors — errors would benefit from a crate like [`anyhow`](https://github.com/dtolnay/anyhow) or
   [`eyre`](https://github.com/eyre-rs/eyre) on the binary side and from [`thiserror`](https://github.com/dtolnay/thiserror)
   on the "library" side (i.e. on error declaration), the errors could be more descriptive and through as well.

4. Better error reporting — The simulator prints the error that occurred and dumps
   the execution trace, this could be made prettier and in conjunction with 1, it could
   show a more complete picture of the test results.

5. Static scenarios — instead of relying just on fuzzing, the tester should have a number of
   static scenarios as a "baseline" for the expected behavior. Just like 3, this would also
   benefit from 1 as we could just add the scenarios to the list of things to test.
