// A list pattern of fixed length, or of a prefix and the rest, as products of `::` ending in `Nil`
// or the list (dotty's `Space.projectSeq`), and a stable identifier as its singleton, which covers
// no value of a `Boolean` or a sealed type (`project` of an `Ident` or a `Select`); the bound's
// children of a type pattern over an abstract type after two cases (`minus`, `decompose`).
object K:
  val b = true
  val n = 1
sealed trait A
case class B() extends A
case class C() extends A

def two(xs: List[Int]): Int = xs match { case List(a, b) => a + b }
def lengths(xs: List[Int]): Int = xs match
  case Nil => 0
  case List(a) => 1
  case List(a, b) => 2
  case List(a, b, rest*) => 3
def heads(xs: List[Int]): Int = xs match
  case List(a, _*) => a
  case Nil => 0
def unreachableList(xs: List[Int]): Int = xs match
  case _ :: _ => 1
  case Nil => 0
  case List(a) => 2
def bool(x: Boolean): Int = x match { case K.b => 1 }
def boolCovered(x: Boolean): Int = x match { case K.b => 1; case true => 2; case false => 3 }
def int(n: Int): Int = n match { case K.n => 1 }
def boundChildren[T <: A](x: A): Int = x match
  case B() => 1
  case _: T => 2
  case C() => 3
def oneBefore[T <: A](x: A): Int = x match
  case _: T => 2
  case C() => 3
  case B() => 1
def typeAfter[T <: A](x: A): Int = x match
  case B() => 1
  case _: T => 2
  case _: C => 3

@main def run(): Unit =
  println(two(List(1, 2)) + lengths(List(1)) + heads(List(4)) + unreachableList(Nil))
  println(bool(true) + boolCovered(false) + int(1))
  println(boundChildren[B](C()) + oneBefore[B](C()) + typeAfter[C](C()))
