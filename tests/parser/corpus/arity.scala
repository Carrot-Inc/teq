// Tuples and functions above the arities the standard library used to spell out.
package arity

import scala.compiletime.{constValue, constValueTuple}
import scala.deriving.Mirror

case class Big(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int, g: Int, h: Int)
case class Pt(x: Int, y: Int)

def eight(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int, g: Int, h: Int): Int = a + b + c + d + e + f + g + h

def sum7(t: (Int, Int, Int, Int, Int, Int, Int)): Int = t._1 + t._2 + t._3 + t._4 + t._5 + t._6 + t._7

extension [A, B, C, D, E, F](t: (Option[A], Option[B], Option[C], Option[D], Option[E], Option[F]))
  def mapN[R](f: (A, B, C, D, E, F) => R): Option[R] =
    for a <- t._1; b <- t._2; c <- t._3; d <- t._4; e <- t._5; g <- t._6 yield f(a, b, c, d, e, g)
  def tupled: Option[(A, B, C, D, E, F)] = mapN((a, b, c, d, e, f) => (a, b, c, d, e, f))

inline def describe[A](using m: Mirror.ProductOf[A]): String =
  constValue[m.MirroredLabel] + constValueTuple[m.MirroredElemLabels].toList.mkString("(", ", ", ")")

@main def main(): Unit =
  val t = (1, 2, 3, 4, 5, 6, 7)
  println(t)
  println(t._7)
  println(sum7(t))
  val (a, b, c, d, e, f, g) = t
  println(a + g)
  val u: (Int, String, Boolean, Double, Long, Char, Unit, List[Int], Option[Int]) =
    (1, "s", true, 2.5, 3L, 'c', (), List(1), Some(2))
  println(u)
  println(u == (1, "s", true, 2.5, 3L, 'c', (), List(1), Some(2)))
  println(u == (1, "s", true, 2.5, 3L, 'c', (), List(1), None))
  println(u.hashCode == (1, "s", true, 2.5, 3L, 'c', (), List(1), Some(2)).hashCode)
  println(Set(t, (1, 2, 3, 4, 5, 6, 7), (7, 6, 5, 4, 3, 2, 1)).size)
  println(Map(t -> "seven").get((1, 2, 3, 4, 5, 6, 7)))

  val fn: (Int, Int, Int, Int, Int, Int, Int, Int) => Int = (a, b, c, d, e, f, g, h) => a + b + c + d + e + f + g + h
  println(fn(1, 2, 3, 4, 5, 6, 7, 8))
  println(List(1, 2).map(Big(_, 1, 2, 3, 4, 5, 6, 7)))
  val mk = Big.apply
  println(mk(1, 2, 3, 4, 5, 6, 7, 8))
  val ef = eight
  println(ef(1, 1, 1, 1, 1, 1, 1, 1))
  val partial = eight(1, 2, 3, _, _, _, 7, 8)
  println(partial(4, 5, 6))
  println((Option(1), Option(2), Option(3), Option(4), Option(5), Option(6)).mapN(_ + _ + _ + _ + _ + _))
  println((Option(1), Option("b"), Option(3.5), Option(true), Option('e'), Option(List(6))).tupled)
  println((Option(1), Option(2), Option(3), Option(4), Option(5), Option(6)).mapN(Pt.apply(_, _).toString + _ + _ + _ + _))
  println((Option(1), Option(2), Option(3), Option(4), Option(5), Option.empty[Int]).tupled)

  t match
    case (1, _, _, _, _, _, x) => println(s"seven ending $x")
    case _ => println("other")
  val xs = List((1, 2, 3, 4, 5, 6), (7, 8, 9, 10, 11, 12))
  println(xs.map((a, b, c, d, e, f) => a + f))
  println(xs.map { case (a, b, c, d, e, f) => b + e })
  println(xs.map(_._6))
  for (a, b, c, d, e, f) <- xs do println(c * d)
  for
    (a, b, c, d, e, f) <- xs
    if a > 1
    (p, q, r, s, v, w) = (f, e, d, c, b, a)
  do println(p + w)
  println(t.copy(_3 = 30))

  val t2: Tuple7[Int, Int, Int, Int, Int, Int, Int] = Tuple7(1, 2, 3, 4, 5, 6, 7)
  println(t2 == t)
  println(scala.Tuple6(1, 2, 3, 4, 5, 6) == (1, 2, 3, 4, 5, 6))
  t2 match
    case Tuple7(x, _, _, _, _, _, y) => println(x + y)
  val f6: Function6[Int, Int, Int, Int, Int, Int, Int] = (a, b, c, d, e, f) => a * f
  println(f6(2, 0, 0, 0, 0, 3))
  val f0: Function0[String] = () => "zero"
  println(f0())
  println(1 -> (2, 3, 4, 5, 6, 7))
  val eighteen = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18)
  println(eighteen._18)
  println(eighteen)
  println(t.productPrefix)
  println(t.toString)
  println((1, 2).swap)
  val any: Any = t
  any match
    case (p, q, r, s, v, w, x) => println(s"a tuple of seven: $p..$x")
    case _ => println("something else")
  println(describe[(Int, String, Boolean, Double, Long, Char, Unit)])
  println(summon[Mirror.ProductOf[(Int, String, Boolean, Double, Long, Char, Unit)]].fromProduct((1, "b", true, 2.5, 5L, 'x', ())))
