package meridian.web.component

import scala.deriving.Mirror
import scala.compiletime.{constValue, erasedValue, summonAll, summonFrom}

/** Whether two values of a hook's dependencies count as the same. */
final class Reusability[A](val test: (A, A) => Boolean):
  def contramap[B](f: B => A): Reusability[B] = new Reusability((x, y) => test(f(x), f(y)))

object Reusability:
  def apply[A](using r: Reusability[A]): Reusability[A] = r
  def by_==[A]: Reusability[A] = new Reusability((x, y) => x == y)
  def byRef[A <: AnyRef]: Reusability[A] = new Reusability((x, y) => x eq y)
  def byRefOr_==[A <: AnyRef]: Reusability[A] = new Reusability((x, y) => (x eq y) || x == y)
  def by[A, B](f: A => B)(using r: Reusability[B]): Reusability[A] = r.contramap(f)
  def always[A]: Reusability[A] = new Reusability((_, _) => true)
  def never[A]: Reusability[A] = new Reusability((_, _) => false)

  given unit: Reusability[Unit] = always
  given int: Reusability[Int] = by_==
  given long: Reusability[Long] = by_==
  given string: Reusability[String] = by_==
  given boolean: Reusability[Boolean] = by_==
  given any[A]: Reusability[A] = by_==
  given option[A](using r: Reusability[A]): Reusability[Option[A]] = new Reusability({
    case (Some(a), Some(b)) => r.test(a, b)
    case (None, None) => true
    case _ => false
  })
  given list[A](using r: Reusability[A]): Reusability[List[A]] =
    new Reusability((xs, ys) => xs.length == ys.length && xs.zip(ys).forall((a, b) => r.test(a, b)))
  given set[A]: Reusability[Set[A]] = by_==
  given map[K, V](using r: Reusability[V]): Reusability[Map[K, V]] =
    new Reusability((xs, ys) => xs.size == ys.size && xs.forall((k, v) => ys.get(k).exists(w => r.test(v, w))))
  given tuple2[A, B](using a: Reusability[A], b: Reusability[B]): Reusability[(A, B)] =
    new Reusability((x, y) => a.test(x._1, y._1) && b.test(x._2, y._2))
  given tuple3[A, B, C](using a: Reusability[A], b: Reusability[B], c: Reusability[C]): Reusability[(A, B, C)] =
    new Reusability((x, y) => a.test(x._1, y._1) && b.test(x._2, y._2) && c.test(x._3, y._3))
  given tuple4[A, B, C, D](using a: Reusability[A], b: Reusability[B], c: Reusability[C], d: Reusability[D]): Reusability[(A, B, C, D)] =
    new Reusability((x, y) => a.test(x._1, y._1) && b.test(x._2, y._2) && c.test(x._3, y._3) && d.test(x._4, y._4))

  inline def derived[A](using m: Mirror.Of[A]): Reusability[A] =
    inline m match
      case p: Mirror.ProductOf[A] =>
        val fields = summonAll[Tuple.Map[p.MirroredElemTypes, Reusability]].toList.asInstanceOf[List[Reusability[Any]]]
        new Reusability((x, y) =>
          val xs = x.asInstanceOf[Product].productIterator.toList
          val ys = y.asInstanceOf[Product].productIterator.toList
          xs.zip(ys).zip(fields).forall { case ((a, b), r) => r.test(a, b) })
      case s: Mirror.SumOf[A] =>
        inline if constValue[Tuple.Size[s.MirroredElemTypes]] > 24 then
          new Reusability((x, y) => s.ordinal(x) == s.ordinal(y))
        else
          val cases = caseInstances[s.MirroredElemTypes]
          new Reusability((x, y) =>
            val i = s.ordinal(x)
            i == s.ordinal(y) && cases(i).test(x, y))
  inline def caseInstances[T <: Tuple]: List[Reusability[Any]] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (a *: b *: c *: d *: t) => summonOrDerive[a].asInstanceOf[Reusability[Any]] :: summonOrDerive[b].asInstanceOf[Reusability[Any]] :: summonOrDerive[c].asInstanceOf[Reusability[Any]] :: summonOrDerive[d].asInstanceOf[Reusability[Any]] :: caseInstances[t]
      case _: (h *: t) => summonOrDerive[h].asInstanceOf[Reusability[Any]] :: caseInstances[t]
  inline def summonOrDerive[T]: Reusability[T] =
    summonFrom {
      case r: Reusability[T] => r
      case m: Mirror.Of[T] => derived[T](using m)
      case _ => by_==[T]
    }
