// A macro's splice runs as the definition typed it, its type arguments inferred there from the
// parameters' declared types (`showImpl('x)` over `x: T` takes `U = T`, over `x: B` takes `B`)
// and its overloads chosen there (`pick('x)` over `x: T` is `pick(Expr[Any])`), the call's type
// arguments substituted, as scalac's inliner keeps the typed splice; the retype path types the
// splice again at the call from the arguments' own types (`Int`, `O.type`). scalac prints the
// lines of the .expected file.
import M.*
@main def run(): Unit =
  println(show[Any](3))
  println(show(3))
  println(showB(O))
  val b: B = O
  println(showB(b))
  println(showInline(O))
  println(showList(List(1, 2)))
  println(showList[Any](List(1, 2)))
  println(pair[AnyVal](1))
  println(pair(true))
  println(which(3) + " " + whichInt(3))
