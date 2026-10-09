package meridian.core.collections

import cats.data.NonEmptyList

/** The non-empty list as cats' NonEmptyList under the name and the surface the model uses (the
  * --cats-data flag of bench/app/gen.py). What NonEmptyList spells differently is added as an
  * extension. */
type Several[+A] = NonEmptyList[A]

object Several:
  def apply[A](head: A, tail: List[A]): Several[A] = NonEmptyList(head, tail)
  def of[A](head: A, tail: A*): Several[A] = NonEmptyList(head, tail.toList)
  def one[A](a: A): Several[A] = NonEmptyList.one(a)
  def fromList[A](list: List[A]): Option[Several[A]] = NonEmptyList.fromList(list)
  def unsafeFromList[A](list: List[A]): Several[A] = NonEmptyList.fromListUnsafe(list)
  def unapply[A](several: Several[A]): Option[(A, List[A])] = Some((several.head, several.tail))

extension [A](several: Several[A])
  def mkString(sep: String): String = several.toList.mkString(sep)
  def minimumBy[B](f: A => B)(using o: Ordering[B]): A = several.toList.minBy(f)
  def maximumBy[B](f: A => B)(using o: Ordering[B]): A = several.toList.maxBy(f)
  def groupBySeveral[K](f: A => K): Map[K, Several[A]] = several.toList.groupBy(f).map((k, v) => (k, NonEmptyList.fromListUnsafe(v)))

extension [A](list: List[A])
  def toSeveral: Option[Several[A]] = NonEmptyList.fromList(list)
  def groupBySeveral[K](f: A => K): Map[K, Several[A]] = list.groupBy(f).map((k, v) => (k, NonEmptyList.fromListUnsafe(v)))
