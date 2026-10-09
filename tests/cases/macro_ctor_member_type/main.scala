// A constructor's method type through memberType, and a type-test pattern built as
// `Typed(Wildcard(), tpt)`, as 3.3 macros spell it.
package app
import mlib.Shapes

final case class Point(x: Int, label: String)

@main def run(): Unit =
  println(Shapes.params[Point])
  println(Shapes.isA[String]("s"))
  println(Shapes.isA[String](3))
