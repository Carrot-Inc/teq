// The patterns of an `inline match` the reduction takes (scalac's `InlineReducer.reducePattern`):
// a case class's pattern by its fields; an extractor of the program's (`Half`, `Even`) and a
// sequence's pattern are not reduced, their case giving way to the next as scalac's does. scalac
// prints the lines of the .expected file.
case class P(a: Int, b: String)
object Even:
  def unapply(n: Int): Boolean = n % 2 == 0
object Half:
  def unapply(n: Int): Option[Int] = Some(n / 2)
inline def kind(inline x: Any): String = inline x match
  case P(n, s) => "P " + n + " " + s
  case Half(h) => "half " + h
  case _ => "other"
inline def even(inline n: Int): String = inline n match
  case Even() => "even"
  case _ => "fallthrough"
inline def seq(inline x: Seq[Int]): String = inline x match
  case Seq(a, b) => "two " + a + b
  case _ => "not reduced"
@main def run(): Unit =
  println(kind(P(1, "x")))
  println(kind(4))
  println(even(4))
  println(seq(Seq(1, 2)))
