package meridian.core.collections

/** A list with at least one element. */
final case class Several[+A](head: A, tail: List[A]):
  def toList: List[A] = head :: tail
  def map[B](f: A => B): Several[B] = Several(f(head), tail.map(f))
  def flatMap[B](f: A => Several[B]): Several[B] =
    val first = f(head)
    Several(first.head, first.tail ++ tail.flatMap(a => f(a).toList))
  def length: Int = tail.length + 1
  def size: Int = length
  def last: A = tail.lastOption.getOrElse(head)
  def prepend[A1 >: A](a: A1): Several[A1] = Several(a, head :: tail)
  def append[A1 >: A](a: A1): Several[A1] = Several(head, tail :+ a)
  def ++[A1 >: A](that: Several[A1]): Several[A1] = Several(head, tail ++ that.toList)
  def find(p: A => Boolean): Option[A] = toList.find(p)
  def exists(p: A => Boolean): Boolean = toList.exists(p)
  def forall(p: A => Boolean): Boolean = toList.forall(p)
  def filter(p: A => Boolean): List[A] = toList.filter(p)
  def foldLeft[B](z: B)(f: (B, A) => B): B = toList.foldLeft(z)(f)
  def reduce[A1 >: A](f: (A1, A1) => A1): A1 = tail.foldLeft[A1](head)(f)
  def zipWithIndex: Several[(A, Int)] = Several((head, 0), tail.zipWithIndex.map((a, i) => (a, i + 1)))
  def sortBy[B](f: A => B)(using o: Ordering[B]): Several[A] = Several.unsafeFromList(toList.sortBy(f))
  def distinct: Several[A] = Several.unsafeFromList(toList.distinct)
  def mkString(sep: String): String = toList.mkString(sep)
  def contains[A1 >: A](a: A1): Boolean = toList.contains(a)
  def minimumBy[B](f: A => B)(using o: Ordering[B]): A = toList.minBy(f)
  def maximumBy[B](f: A => B)(using o: Ordering[B]): A = toList.maxBy(f)
  def groupBy[K](f: A => K): Map[K, Several[A]] = toList.groupBy(f).map((k, v) => (k, Several.unsafeFromList(v)))

object Several:
  def of[A](head: A, tail: A*): Several[A] = Several(head, tail.toList)
  def one[A](a: A): Several[A] = Several(a, Nil)
  def fromList[A](list: List[A]): Option[Several[A]] = list match
    case h :: t => Some(Several(h, t))
    case Nil => None
  def unsafeFromList[A](list: List[A]): Several[A] = fromList(list).getOrElse(throw new IllegalArgumentException("Several cannot be empty"))

extension [A](list: List[A])
  def toSeveral: Option[Several[A]] = Several.fromList(list)
  def groupBySeveral[K](f: A => K): Map[K, Several[A]] = list.groupBy(f).map((k, v) => (k, Several.unsafeFromList(v)))
