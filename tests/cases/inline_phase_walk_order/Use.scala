// The inlining phase expands the calls in the order of the program's trees (`InliningTreeMap`), a
// member's after the members before it, an argument's call before the call, whatever order the typer
// met them in: `a`'s right-hand side types `b`'s body on demand for its inferred type, yet `a`'s macro
// runs first; within a member, the arguments before the call, in their order.
object O:
  val a = b + P.next * 10
  def b = P.next * 100
  val c = P.pair(P.next, P.next)
@main def run(): Unit =
  println(O.a)
  println(O.b)
  println(O.c)
