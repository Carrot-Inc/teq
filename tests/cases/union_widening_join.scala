// A union the typer joins (the branches of an `if`, the elements of a `List`, a lambda's
// branches) widens to the class over the bounded wildcard where a type is inferred, as dotc's
// `widenInferred`: the `Box[?]` overload is picked over the union's and a `Box[Int] |
// Box[String]` is not what the inferred value has; a union of unrelated types stays a union
// (its join is transparent), and a union written in the program is kept.
import scala.annotation.targetName
class Box[A](val a: A)

def value(b: Boolean) = if b then new Box[Int](1) else new Box[String]("s")
def pick(x: Box[Int] | Box[String]): String = "union"
@targetName("pickWild")
def pick(x: Box[?]): String = "wild"
def describe(x: Int | String): String = x match
  case i: Int => "int " + i
  case s: String => "string " + s

@main def run(): Unit =
  println(pick(value(true)))
  val v = value(false)
  println(pick(v))
  val u: Box[Int] | Box[String] = new Box[Int](2)
  val u2 = u
  println(pick(u2))
  val w = if "ab".length > 1 then 1 else "a"
  println(describe(w))
  val l = List(new Box[Int](3), new Box[String]("t"))
  println(l.map(_.a).mkString(","))
  val f = (b: Boolean) => if b then new Box[Int](4) else new Box[String]("u")
  println(pick(f(true)))
