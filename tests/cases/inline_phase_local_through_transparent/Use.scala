// A local plain inline method called from a transparent method's expansion is kept for the inlining
// phase as any plain call is (`Inlines.needsInlining`: one decision in the typer and inside a
// transparent expansion), so the counter macro it calls runs in the phase, after the plain call
// before it in the tree: scalac prints 1,2 (an expansion at the transparent call's typing gave 2,1).
@main def run(): Unit =
  inline def local: Int = M.next
  transparent inline def outer: Int = local
  val a = M.next
  val b = outer
  println(s"$a,$b")
