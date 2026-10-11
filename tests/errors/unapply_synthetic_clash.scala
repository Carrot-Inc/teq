// expect: 19:29: error: Ambiguous overload. The overloaded alternatives of method unapply in object C with types
// expect:  (c: => C): Option[Int]
// expect:  (x$1: C): C
// expect: 21:35: error: Ambiguous overload. The overloaded alternatives of method unapply in object Box with types
// expect:  [A](x$1: Box[A]): Box[A]
// expect:  (b: Box[String]): Option[Int]
// expect: 2 errors found

// A case class's synthetic `unapply` keeps its whole signature, a by-name parameter's mode
// included (`matchingMethodParams`), and its companion as owner (`compareWithTypes`).
case class C(x: Int)
object C:
  def unapply(c: => C): Option[Int] = Some(c.x + 10)
case class Box[A](x: A)
trait Extract:
  def unapply(b: Box[String]): Option[Int] = Some(99)
object Box extends Extract
@main def run(): Unit =
  println(C(3) match { case C(x) => x })
  println(Box(23) match { case Box(x) => x })
  println(Box("abc") match { case Box(x) => x })
