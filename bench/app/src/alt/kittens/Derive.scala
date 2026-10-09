package meridian.core.derive

import cats.derived.semiauto

import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple, erasedValue, summonAll, summonFrom}

/** The derivations of Eq, Show, Order and Monoid through kittens' semiauto (the --kittens flag
  * of bench/app/gen.py), behind the same type classes and container instances. */
trait Eq[A] extends cats.Eq[A]
object Eq:
  def apply[A](using e: cats.Eq[A]): cats.Eq[A] = e
  def fromUniversalEquals[A]: Eq[A] = (x, y) => x == y
  def by[A, B](f: A => B)(using b: cats.Eq[B]): Eq[A] = (x, y) => b.eqv(f(x), f(y))
  def instance[A](f: (A, A) => Boolean): Eq[A] = (x, y) => f(x, y)
  inline def derived[A](using m: Mirror.Of[A]): Eq[A] =
    val k = semiauto.eq[A]
    (x, y) => k.eqv(x, y)
  inline def caseInstances[T <: Tuple]: List[cats.Eq[Any]] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (a *: b *: c *: d *: t) => summonOrDerive[a].asInstanceOf[cats.Eq[Any]] :: summonOrDerive[b].asInstanceOf[cats.Eq[Any]] :: summonOrDerive[c].asInstanceOf[cats.Eq[Any]] :: summonOrDerive[d].asInstanceOf[cats.Eq[Any]] :: caseInstances[t]
      case _: (h *: t) => summonOrDerive[h].asInstanceOf[cats.Eq[Any]] :: caseInstances[t]
  inline def summonOrDerive[T]: cats.Eq[T] =
    summonFrom {
      case e: cats.Eq[T] => e
      case m: Mirror.Of[T] => derived[T](using m)
    }
  given derivedList[A](using e: cats.Eq[A]): cats.Eq[List[A]] = Elementwise.listEq(e)
  given derivedVector[A](using e: cats.Eq[A]): cats.Eq[Vector[A]] = Elementwise.vectorEq(e)
  given derivedOption[A](using e: cats.Eq[A]): cats.Eq[Option[A]] = Elementwise.optionEq(e)
  given derivedSet[A]: cats.Eq[Set[A]] = Elementwise.setEq[A]
  given derivedMap[K, V](using e: cats.Eq[V]): cats.Eq[Map[K, V]] = Elementwise.mapEq(e)
  given derivedEither[L, R](using l: cats.Eq[L], r: cats.Eq[R]): cats.Eq[Either[L, R]] = Elementwise.eitherEq(l, r)
  given derivedTuple2[A, B](using a: cats.Eq[A], b: cats.Eq[B]): cats.Eq[(A, B)] = Elementwise.tuple2Eq(a, b)

trait Show[A] extends cats.Show[A]
object Show:
  def apply[A](using s: cats.Show[A]): cats.Show[A] = s
  def show[A](f: A => String): Show[A] = a => f(a)
  def fromToString[A]: Show[A] = a => a.toString
  inline def derived[A](using m: Mirror.Of[A]): Show[A] =
    val k = semiauto.show[A]
    a => k.show(a)
  inline def caseInstances[T <: Tuple]: List[cats.Show[Any]] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (a *: b *: c *: d *: t) => summonOrDerive[a].asInstanceOf[cats.Show[Any]] :: summonOrDerive[b].asInstanceOf[cats.Show[Any]] :: summonOrDerive[c].asInstanceOf[cats.Show[Any]] :: summonOrDerive[d].asInstanceOf[cats.Show[Any]] :: caseInstances[t]
      case _: (h *: t) => summonOrDerive[h].asInstanceOf[cats.Show[Any]] :: caseInstances[t]
  inline def summonOrDerive[T]: cats.Show[T] =
    summonFrom {
      case s: cats.Show[T] => s
      case m: Mirror.Of[T] => derived[T](using m)
    }
  given derivedList[A](using s: cats.Show[A]): cats.Show[List[A]] = Elementwise.listShow(s)
  given derivedVector[A](using s: cats.Show[A]): cats.Show[Vector[A]] = Elementwise.vectorShow(s)
  given derivedOption[A](using s: cats.Show[A]): cats.Show[Option[A]] = Elementwise.optionShow(s)
  given derivedSet[A](using s: cats.Show[A]): cats.Show[Set[A]] = Elementwise.setShow(s)
  given derivedMap[K, V](using k: cats.Show[K], v: cats.Show[V]): cats.Show[Map[K, V]] = Elementwise.mapShow(k, v)
  given derivedTuple2[A, B](using a: cats.Show[A], b: cats.Show[B]): cats.Show[(A, B)] = Elementwise.tuple2Show(a, b)

trait Order[A] extends cats.Order[A]
object Order:
  def apply[A](using o: cats.Order[A]): cats.Order[A] = o
  def by[A, B](f: A => B)(using b: cats.Order[B]): Order[A] = (x, y) => b.compare(f(x), f(y))
  def fromLessThan[A](lt: (A, A) => Boolean): Order[A] = (x, y) => if lt(x, y) then -1 else if lt(y, x) then 1 else 0
  def fromOrdering[A](using o: Ordering[A]): Order[A] = (x, y) => o.compare(x, y)
  inline def derived[A](using m: Mirror.Of[A]): Order[A] =
    val k = semiauto.order[A]
    (x, y) => k.compare(x, y)
  inline def caseInstances[T <: Tuple]: List[cats.Order[Any]] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (a *: b *: c *: d *: t) => summonOrDerive[a].asInstanceOf[cats.Order[Any]] :: summonOrDerive[b].asInstanceOf[cats.Order[Any]] :: summonOrDerive[c].asInstanceOf[cats.Order[Any]] :: summonOrDerive[d].asInstanceOf[cats.Order[Any]] :: caseInstances[t]
      case _: (h *: t) => summonOrDerive[h].asInstanceOf[cats.Order[Any]] :: caseInstances[t]
  inline def summonOrDerive[T]: cats.Order[T] =
    summonFrom {
      case o: cats.Order[T] => o
      case m: Mirror.Of[T] => derived[T](using m)
    }
  given derivedList[A](using o: cats.Order[A]): cats.Order[List[A]] = Elementwise.listOrder(o)
  given derivedOption[A](using o: cats.Order[A]): cats.Order[Option[A]] = Elementwise.optionOrder(o)
  given derivedTuple2[A, B](using a: cats.Order[A], b: cats.Order[B]): cats.Order[(A, B)] = Elementwise.tuple2Order(a, b)

trait Monoid[A] extends cats.Monoid[A]
object Monoid:
  def apply[A](using m: cats.Monoid[A]): cats.Monoid[A] = m
  def instance[A](emptyValue: A, cmb: (A, A) => A): Monoid[A] = new Monoid[A]:
    def empty = emptyValue
    def combine(x: A, y: A) = cmb(x, y)
  inline def derived[A](using m: Mirror.ProductOf[A]): Monoid[A] =
    val k = semiauto.monoid[A]
    new Monoid[A]:
      def empty = k.empty
      def combine(x: A, y: A) = k.combine(x, y)
/** The instances of the standard containers written out, so that a derivation reaches the
  * element through its own search and never through a mirror of the container. */
object Elementwise:
  def listEq[A](e: cats.Eq[A]): cats.Eq[List[A]] = (xs, ys) => xs.length == ys.length && xs.zip(ys).forall((a, b) => e.eqv(a, b))
  def vectorEq[A](e: cats.Eq[A]): cats.Eq[Vector[A]] = (xs, ys) => xs.length == ys.length && xs.zip(ys).forall((a, b) => e.eqv(a, b))
  def optionEq[A](e: cats.Eq[A]): cats.Eq[Option[A]] = (x, y) => (x, y) match
    case (Some(a), Some(b)) => e.eqv(a, b)
    case (None, None) => true
    case _ => false
  def setEq[A]: cats.Eq[Set[A]] = (xs, ys) => xs == ys
  def mapEq[K, V](e: cats.Eq[V]): cats.Eq[Map[K, V]] = (xs, ys) => xs.size == ys.size && xs.forall((k, v) => ys.get(k).exists(w => e.eqv(v, w)))
  def eitherEq[L, R](l: cats.Eq[L], r: cats.Eq[R]): cats.Eq[Either[L, R]] = (x, y) => (x, y) match
    case (Left(a), Left(b)) => l.eqv(a, b)
    case (Right(a), Right(b)) => r.eqv(a, b)
    case _ => false
  def tuple2Eq[A, B](a: cats.Eq[A], b: cats.Eq[B]): cats.Eq[(A, B)] = (x, y) => a.eqv(x._1, y._1) && b.eqv(x._2, y._2)

  def listShow[A](s: cats.Show[A]): cats.Show[List[A]] = xs => xs.map(s.show).mkString("List(", ", ", ")")
  def vectorShow[A](s: cats.Show[A]): cats.Show[Vector[A]] = xs => xs.map(s.show).mkString("Vector(", ", ", ")")
  def optionShow[A](s: cats.Show[A]): cats.Show[Option[A]] = o => o match
    case Some(a) => s"Some(${s.show(a)})"
    case None => "None"
  def setShow[A](s: cats.Show[A]): cats.Show[Set[A]] = xs => xs.toList.map(s.show).mkString("Set(", ", ", ")")
  def mapShow[K, V](k: cats.Show[K], v: cats.Show[V]): cats.Show[Map[K, V]] = m => m.toList.map((a, b) => s"${k.show(a)} -> ${v.show(b)}").mkString("Map(", ", ", ")")
  def tuple2Show[A, B](a: cats.Show[A], b: cats.Show[B]): cats.Show[(A, B)] = t => s"(${a.show(t._1)},${b.show(t._2)})"

  def listOrder[A](o: cats.Order[A]): cats.Order[List[A]] = (xs, ys) =>
    xs.zip(ys).iterator.map((a, b) => o.compare(a, b)).find(_ != 0).getOrElse(Integer.compare(xs.length, ys.length))
  def optionOrder[A](o: cats.Order[A]): cats.Order[Option[A]] = (x, y) => (x, y) match
    case (Some(a), Some(b)) => o.compare(a, b)
    case (None, None) => 0
    case (None, _) => -1
    case _ => 1
  def tuple2Order[A, B](a: cats.Order[A], b: cats.Order[B]): cats.Order[(A, B)] = (x, y) =>
    val first = a.compare(x._1, y._1)
    if first != 0 then first else b.compare(x._2, y._2)

final class Elems(values: List[Any]) extends Product:
  private val array = values.toArray
  def productArity = array.length
  def productElement(n: Int) = array(n)
  def canEqual(that: Any) = false
