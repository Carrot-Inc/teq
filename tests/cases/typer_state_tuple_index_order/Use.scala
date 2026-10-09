// A tuple's index over pending plain calls of constant types folds by their types and leaves the
// calls to expand in the order they were typed: `one` expands after
// the `wide` before it, in the index or before it, as in scalac's `Inlining` phase, and an index
// whose expansion has an effect is evaluated after the receiver and before the element is read.
class R:
  println("receiver")
  val x: 0 = 0

object Obj:
  println("module")

@main def run(): Unit =
  val t = ("zero", "one", "two")
  println(t(A.wide + A.one))
  val b = B.wide
  println(t(B.one))
  println(b)
  val c = C.wide
  val s: String = t(C.one + 0)
  println(s + c)
  println(("x", "y")(C.side + 0))
  println(t(-C.one + 1))
  println(((new R).x, "b")(C.side + 0))
  println((Obj, "b")(C.side + 0))
