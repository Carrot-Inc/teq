object A:
  class Money(val n: Int)
  class Box(val v: Int)
  opaque type M = Money
def uses(b: A.Box): Unit =
  println(b match { case _: A.M => 1; case _ => 0 })
// A control beside a class named through a path: an opaque type named from outside its scope is written with a
// prefix too and is no class there; its unrelated typed pattern is told as before, at scalac's place.
// expect: 6:26: error: unreachable case: type Box and class Money are unrelated
// expect: 1 error found
