// expect: ambiguous given instances for A => K1 | K2: aToK1, aToK2
// Two conversions whose results are members of the union apply, and a third whose result is no
// member's does not take part: the ambiguity names the two.
import scala.language.implicitConversions

final class A
final class K1
final class K2
final class B

object Convs:
  implicit def aToK1(a: A): K1 = K1()
  implicit def aToB(a: A): B = B()
  implicit def aToK2(a: A): K2 = K2()

import Convs.*

def pick[T](t: T)(using f: T => (K1 | K2)): String = f(t).toString

@main def main(): Unit =
  println(pick(A()))
