// A plain call in a transparent expansion stays a call in the typer, as dotty's `InlineTyper`
// keeps it (`Inlines.needsInlining` holds in the typer for a transparent method or under
// `Mode.ForceInline` alone), and the inlining phase expands it in its tree's order: `a`'s macro
// runs before `b`'s nested one, which the typer met first. An `inline if`'s condition is forced.
object O:
  val a = P.next * 10
  val b = P.viaTransparent * 100
  val c = P.forced
@main def run(): Unit =
  println(O.a)
  println(O.b)
  println(O.c)
