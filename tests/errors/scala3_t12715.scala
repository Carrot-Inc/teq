// Adapted from scala3 tests/neg/t12715.scala (Apache-2.0, see tests/scala3/README.md); replaced: the main object is dropped.
// expect: 10:13: error: parent trait E has a super call which binds to the value D.f. Super calls can only target methods.
// expect: 11:13: error: parent trait E has a super call which binds to the value C.f. Super calls can only target methods.
// expect: 2 errors found
trait A              { def f: String }
trait B extends A    { def f = "B" }
trait C extends A    { override val f = "C" }
trait D extends C    { override val f = "D" }
trait E extends A, B { def d = super.f }
final class O1 extends B, C, D, E
final class O2 extends B, C, E, D
final class O3 extends B, E, C, D
