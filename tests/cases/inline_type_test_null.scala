// A type pattern over the abstract type its scrutinee has, in an inline body the expansion fills with
// a type that takes `null`: dotty's `TypeTestsCasts.transformIsInstanceOf` makes a match's test of a
// conforming scrutinee `true` only where the scrutinee's type is not null (`isNotNull`), else a test
// that it is not null (`testNotNull`), so a `null` String takes the default case and an `Int` the
// first. The stored body's test is that null test, whatever the expansion fills `A` with.
inline def matches[A](a: A, ok: Boolean): Boolean = a match
  case _: A if ok => true
  case _ => false
inline def tester[A](ok: Boolean): A => Boolean = (x: A) => x match
  case _: A if ok => true
  case _ => false
def plain(s: String): Boolean = s match
  case _: String => true
  case null => false
@main def run(): Unit =
  val none: String = null
  println(s"${matches[String](none, true)},${matches[String]("s", true)},${matches[Int](3, true)}")
  println(s"${tester[String](true)(none)},${tester[String](true)("s")}")
  println(s"${plain(none)},${plain("s")}")
