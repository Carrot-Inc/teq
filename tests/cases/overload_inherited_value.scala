// A method overloading an inherited val and lazy val of its name with other parameters: the
// class's entry holds both (dotty's `membersNamed` keeps every member no other matches).
trait T { val x: Int = 4; lazy val y: Int = 5 }
class C extends T { def x(s: String): String = s + x; def y(i: Int): Int = i + y }
@main def run(): Unit =
  val c = new C
  println(c.x); println(c.x("s")); println((c: T).x); println(c.y(1))
