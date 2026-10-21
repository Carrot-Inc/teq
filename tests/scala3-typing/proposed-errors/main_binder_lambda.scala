// checkAnnotApplicable gives @main on a binder two errors at one place; scalac 3.8.4's reporter hides the second
// (UniqueMessagePositions), so it prints one. Pinned for the part that ports that hiding (the ruling on report §8).
// expect: 5:22: error: main annotation cannot be applied to type A
// absent: cannot be a main method since it cannot be accessed statically
object O { type L = [@main A] =>> Int }
