// A Java annotation's argument is typed against its element's formal (`matchArgs` → `addTyped` → `typedArg`), a nested
// annotation by its own application and then adapted to the formal; a nested one that fails is an error tree the outer
// matching goes on past (the review of `b23cfe2a3`: `JavaNestedWrong`, `JavaNestedNamedWrong`, `J1`). scalac 3.8.4
// gives these four errors.
// expect: 9:40: error: type mismatch: found Deprecated, required Array[String]
// expect: 10:42: error: type mismatch: found Deprecated, required String
// expect: 11:67: error: constructor Deprecated in trait Deprecated: (since: String, forRemoval: Boolean): Deprecated does not have a parameter foo
// expect: 11:92: error: type mismatch: found "y", required Boolean
type L1 = [@java.lang.SuppressWarnings(new java.lang.Deprecated()) A] =>> List[A]
type L2 = [@java.lang.Deprecated(since = new java.lang.Deprecated()) A] =>> List[A]
type J1 = [@java.lang.Deprecated(since = new java.lang.Deprecated(foo = "x"), forRemoval = "y") A] =>> List[A]
