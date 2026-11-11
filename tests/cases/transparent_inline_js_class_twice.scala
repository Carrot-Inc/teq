//> using platform js
// A transparent inline method whose body defines a class extending `js.Object`, expanded twice: the JavaScript
// preparation, run after the workers' typing, prepares each expansion's copy of the class by the copy's own members,
// and no copy finds the mark of another's as one the program wrote (PrepJSInterop's `transformMemberDef` on each
// definition's own symbol).
import scala.scalajs.js

object Factory:
  transparent inline def make(n: Int): js.Object =
    class Box extends js.Object:
      def value: Int = n
      def twice: Int = n * 2
    new Box

object Use:
  val a: js.Object = Factory.make(3)
  val b: js.Object = Factory.make(4)

@main def run(): Unit =
  println(Use.a != null)
  println(Use.b ne Use.a)
