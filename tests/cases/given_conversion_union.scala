// A conversion method is a candidate for an implicit function value whose result is a union only
// where its result can be a member's: `aToB` cannot, `aToSubK1` can through `K1`, and
// `Predef.conforms` does not fit. A tuple conforms to a tuple type of another shape (`cToPair`'s
// `(Int, String)` to `Int *: String *: EmptyTuple`).
import scala.language.implicitConversions

final class A
class K1(val s: String)
final class SubK1 extends K1("sub")
final class K2
final class B

object Convs:
  implicit def aToB(a: A): B = B()
  implicit def aToSubK1(a: A): SubK1 = SubK1()

import Convs.*

final class C
object TupleConvs:
  implicit def cToPair(c: C): (Int, String) = (1, "pair")
  implicit def cToB(c: C): B = B()

import TupleConvs.*

def pickTuple[T](t: T)(using f: T => (K2 | (Int *: String *: EmptyTuple))): String = f(t) match
  case _: K2 => "k2"
  case other => other.toString

def pick[T](t: T)(using f: T => (K1 | K2)): String = f(t) match
  case k: K1 => k.s
  case _: K2 => "k2"

@main def main(): Unit =
  println(pick(A()))
  println(pick(A()))
  println(pickTuple(C()))
