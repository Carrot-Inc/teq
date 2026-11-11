package rmr

import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple, erasedValue, summonAll, summonFrom}

trait Show[A]:
  def show(a: A): String

object Show:
  given Show[String] = a => a
  given Show[Long] = a => a.toString

  inline def derived[A](using m: Mirror.Of[A]): Show[A] =
    inline m match
      case p: Mirror.ProductOf[A] =>
        val name = constValue[p.MirroredLabel]
        val labels = constValueTuple[p.MirroredElemLabels].toList.asInstanceOf[List[String]]
        val shows = summonAll[Tuple.Map[p.MirroredElemTypes, Show]].toList.asInstanceOf[List[Show[Any]]]
        a => name + labels.zip(a.asInstanceOf[Product].productIterator.toList.zip(shows)).map { case (l, (v, s)) => l + "=" + s.show(v) }.mkString("(", ",", ")")
      case s: Mirror.SumOf[A] =>
        val cases = forCases[s.MirroredElemTypes]
        a => cases(s.ordinal(a)).show(a)

  inline def forCases[T <: Tuple]: List[Show[Any]] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (h *: t) => givenOrDerived[h].asInstanceOf[Show[Any]] :: forCases[t]

  inline def givenOrDerived[T]: Show[T] =
    summonFrom {
      case s: Show[T] => s
      case m: Mirror.Of[T] => derived[T](using m)
    }
