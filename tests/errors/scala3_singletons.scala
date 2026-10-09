// Adapted from scala3 tests/neg/singletons.scala (Apache-2.0, see tests/scala3/README.md): the constant-type cases.
// expect: 5:15: error: type mismatch: found 43, required 42
// expect: 7:15: error: type mismatch: found Int, required 42
object Test:
  val a: 42 = 43  // error: different constant
  val x = 42
  val z: 42 = x   // error: x is not final
