package scala

final class ArraySeq[+A](private val items: Array[A]) extends SeqOps[A, ArraySeq, ArraySeq[A]], IndexedSeq[A]:
  def buildCC[B](items: RawBuffer[B]): ArraySeq[B] = new ArraySeq(untaggedArray(items))
  override def iterator: Iterator[A] = arrayIterator(items)
  def length: Int = items.length
  def apply(i: Int): A =
    if i < 0 || i >= items.length then indexOutOfBounds(i.toString)
    else items(i)
  def foreach[U](f: A => U): Unit =
    var i = 0
    while i < items.length do
      f(items(i))
      i += 1
  def toList: List[A] =
    var acc: List[A] = Nil
    var i = items.length - 1
    while i >= 0 do
      acc = items(i) :: acc
      i -= 1
    acc
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(items.clone())
  // scala-library's view of the array itself and its element tag (`unsafeArray` and `elemTag`
  // on the immutable one, `array` on the mutable one); the tag is the reference one, arrays
  // being untyped here.
  def unsafeArray: Array[?] = items
  def array: Array[?] = items
  def elemTag: scala.reflect.ClassTag[?] = scala.reflect.ClassTag.AnyRef
  override def collectionClassName: String = "ArraySeq"
  override def toString: String = mkString("ArraySeq(", ", ", ")")

object ArraySeq:
  @jvmEvidence
  def apply[A: scala.reflect.ClassTag](elems: A*): ArraySeq[A] = new ArraySeq(taggedArray(iterableToArray(elems)))
  @jvmEvidence
  def empty[A: scala.reflect.ClassTag]: ArraySeq[A] = new ArraySeq(taggedArray(emptyBuffer[A]))
  @jvmEvidence
  def from[A: scala.reflect.ClassTag](source: IterableOnce[A]): ArraySeq[A] = new ArraySeq(taggedArray(iterableToArray(source)))
  def unsafeWrapArray[A](x: Array[A]): ArraySeq[A] = new ArraySeq(x)
  // scala-library's companion is a `ClassTagSeqFactory`, whose evidence factory a library body
  // names as the implicit `Factory` it resolved (izumi-reflect's picklers).
  implicit def evidenceIterableFactory[A](implicit ev: scala.reflect.ClassTag[A]): Factory[A, ArraySeq[A]] =
    new Factory[A, ArraySeq[A]]:
      def fromSpecific(it: IterableOnce[A]): ArraySeq[A] = from(it)
      def newBuilder: scala.collection.mutable.Builder[A, ArraySeq[A]] = scala.collection.mutable.ArrayBuffer.empty[A].mapResult(b => from(b))

sealed trait Option[+A] extends IterableOnce[A], Product:
  def isEmpty: Boolean = this match
    case None => true
    case _ => false
  def isDefined: Boolean = !isEmpty
  def nonEmpty: Boolean = !isEmpty
  def get: A = this match
    case Some(v) => v
    case None => noSuchElement("None.get")
  def getOrElse[B >: A](default: => B): B = this match
    case Some(v) => v
    case None => default
  def orElse[B >: A](alternative: => Option[B]): Option[B] = this match
    case Some(_) => this
    case None => alternative
  def map[B](f: A => B): Option[B] = this match
    case Some(v) => Some(f(v))
    case None => None
  def flatMap[B](f: A => Option[B]): Option[B] = this match
    case Some(v) => f(v)
    case None => None
  def filter(p: A => Boolean): Option[A] = this match
    case Some(v) if p(v) => this
    case _ => None
  def withFilter(p: A => Boolean): Option[A] = filter(p)
  def collect[B](pf: PartialFunction[A, B]): Option[B] = this match
    case Some(v) =>
      val r = pf.applyOrElse(v, partialMiss[A, B])
      if isPartialMiss(r) then None else Some(r)
    case None => None
  def foreach[U](f: A => U): Unit = this match
    case Some(v) =>
      f(v)
      ()
    case None => ()
  def fold[B](ifEmpty: => B)(f: A => B): B = this match
    case Some(v) => f(v)
    case None => ifEmpty
  def exists(p: A => Boolean): Boolean = this match
    case Some(v) => p(v)
    case None => false
  def forall(p: A => Boolean): Boolean = this match
    case Some(v) => p(v)
    case None => true
  def contains[B >: A](elem: B): Boolean = this match
    case Some(v) => v == elem
    case None => false
  def toList: List[A] = this match
    case Some(v) => v :: Nil
    case None => Nil
  def filterNot(p: A => Boolean): Option[A] = filter(x => !p(x))
  def find(p: A => Boolean): Option[A] = filter(p)
  def count(p: A => Boolean): Int = if exists(p) then 1 else 0
  def foldLeft[B](z: B)(op: (B, A) => B): B = this match
    case Some(v) => op(z, v)
    case None => z
  def head: A = this match
    case Some(v) => v
    case None => noSuchElement("head of empty list")
  def headOption: Option[A] = this
  def lastOption: Option[A] = this
  def toRight[X](left: => X): Either[X, A] = this match
    case Some(v) => Right(v)
    case None => Left(left)
  def toLeft[X](right: => X): Either[A, X] = this match
    case Some(v) => Left(v)
    case None => Right(right)
  def zip[B](that: Option[B]): Option[(A, B)] = (this, that) match
    case (Some(a), Some(b)) => Some((a, b))
    case _ => None
  def size: Int = if isEmpty then 0 else 1
  def knownSize: Int = size
  def toSeq: Seq[A] = toList
  def toVector: Vector[A] = toList.toVector
  def toSet[B >: A]: Set[B] = Set.from(this)

final case class Some[+A](value: A) extends Option[A]
case object None extends Option[Nothing]

object Option:
  def apply[A](value: A): Option[A] = if js.isNull(value) || js.isUndefined(value) then None else Some(value)
  def empty[A]: Option[A] = None
  implicit def option2Iterable[A](xo: Option[A]): Iterable[A] = xo.toList
  def when[A](condition: Boolean)(value: => A): Option[A] = if condition then Some(value) else None
  def unless[A](condition: Boolean)(value: => A): Option[A] = if condition then None else Some(value)

extension [A](o: Option[A])
  def flatten[B](implicit ev: A <:< Option[B]): Option[B] = o match
    case Some(inner) => ev(inner)
    case None => None
  def orNull[A1 >: A](implicit ev: Null <:< A1): A1 = o match
    case Some(v) => v
    case None => ev(null)

extension [A, B](o: Option[(A, B)])
  def unzip: (Option[A], Option[B]) = o match
    case Some((a, b)) => (Some(a), Some(b))
    case None => (None, None)

sealed trait Either[+A, +B] extends Product:
  def isLeft: Boolean = this match
    case Left(_) => true
    case Right(_) => false
  def isRight: Boolean = !isLeft
  def map[C](f: B => C): Either[A, C] = this match
    case Right(v) => Right(f(v))
    case Left(e) => Left(e)
  def flatMap[A1 >: A, C](f: B => Either[A1, C]): Either[A1, C] = this match
    case Right(v) => f(v)
    case Left(e) => Left(e)
  def fold[C](onLeft: A => C, onRight: B => C): C = this match
    case Left(e) => onLeft(e)
    case Right(v) => onRight(v)
  def getOrElse[B1 >: B](default: => B1): B1 = this match
    case Right(v) => v
    case Left(_) => default
  def toOption: Option[B] = this match
    case Right(v) => Some(v)
    case Left(_) => None
  def foreach[U](f: B => U): Unit = this match
    case Right(v) =>
      f(v)
      ()
    case Left(_) => ()
  def left: LeftProjection[A, B] = new LeftProjection(this)
  def right: RightProjection[A, B] = new RightProjection(this)
  def swap: Either[B, A] = this match
    case Left(e) => Right(e)
    case Right(v) => Left(v)
  def orElse[A1 >: A, B1 >: B](alternative: => Either[A1, B1]): Either[A1, B1] = this match
    case Right(_) => this
    case Left(_) => alternative
  def exists(p: B => Boolean): Boolean = this match
    case Right(v) => p(v)
    case Left(_) => false
  def forall(p: B => Boolean): Boolean = this match
    case Right(v) => p(v)
    case Left(_) => true
  def contains[B1 >: B](elem: B1): Boolean = this match
    case Right(v) => v == elem
    case Left(_) => false
  def filterOrElse[A1 >: A](p: B => Boolean, zero: => A1): Either[A1, B] = this match
    case Right(v) if !p(v) => Left(zero)
    case _ => this
  def toSeq: Seq[B] = this match
    case Right(v) => v :: Nil
    case Left(_) => Nil

final case class Left[+A, +B](value: A) extends Either[A, B]
final case class Right[+A, +B](value: B) extends Either[A, B]

final class LeftProjection[+A, +B](e: Either[A, B]):
  def toOption: Option[A] = e match
    case Left(v) => Some(v)
    case Right(_) => None
  def toSeq: Seq[A] = toOption.toList
  def map[A1](f: A => A1): Either[A1, B] = e match
    case Left(v) => Left(f(v))
    case Right(v) => Right(v)
  def flatMap[A1, B1 >: B](f: A => Either[A1, B1]): Either[A1, B1] = e match
    case Left(v) => f(v)
    case Right(v) => Right(v)
  def getOrElse[A1 >: A](default: => A1): A1 = toOption.getOrElse(default)
  def foreach[U](f: A => U): Unit = toOption.foreach(f)
  def exists(p: A => Boolean): Boolean = toOption.exists(p)
  def forall(p: A => Boolean): Boolean = toOption.forall(p)

final class RightProjection[+A, +B](e: Either[A, B]):
  def toOption: Option[B] = e.toOption
  def toSeq: Seq[B] = e.toSeq
  def map[B1](f: B => B1): Either[A, B1] = e.map(f)
  def flatMap[A1 >: A, B1](f: B => Either[A1, B1]): Either[A1, B1] = e.flatMap(f)
  def getOrElse[B1 >: B](default: => B1): B1 = e.getOrElse(default)
  def foreach[U](f: B => U): Unit = e.foreach(f)
  def exists(p: B => Boolean): Boolean = e.exists(p)
  def forall(p: B => Boolean): Boolean = e.forall(p)
  def get: B = e match
    case Right(v) => v
    case Left(_) => noSuchElement("Either.right.get on Left")

object Either:
  def cond[A, B](test: Boolean, right: => B, left: => A): Either[A, B] = if test then Right(right) else Left(left)
  // What a library body calls for `merge` (`Either.MergeableEither(e).merge`).
  implicit class MergeableEither[A](x: Either[A, A]):
    def merge: A = x match
      case Left(v) => v
      case Right(v) => v

extension [A, B](e: Either[A, B])
  def toTry(implicit ev: A <:< Throwable): scala.util.Try[B] = e match
    case Right(v) => scala.util.Success(v)
    case Left(t) => scala.util.Failure(ev(t))

extension [A](e: Either[A, A])
  def merge: A = e match
    case Left(v) => v
    case Right(v) => v

extension [A, B](e: Either[A, B])
  def flatten[A1 >: A, B1](implicit ev: B <:< Either[A1, B1]): Either[A1, B1] = e match
    case Right(inner) => ev(inner)
    case Left(v) => Left(v)

sealed trait List[+A] extends SeqOps[A, List, List[A]], Seq[A]:
  def isEmpty: Boolean
  def head: A
  def tail: List[A]

  def buildCC[B](items: RawBuffer[B]): List[B] = fromArray(items)
  override def iterator: Iterator[A] =
    var cur: List[A] = this
    def step(): A =
      if cur.isEmpty then Iterator.exhausted
      val x = cur.head
      cur = cur.tail
      x
    new FnIterator(() => !cur.isEmpty, () => step())

  def ::[B >: A](elem: B): List[B] = new ::(elem, this)

  def length: Int =
    var n = 0
    var cur: List[A] = this
    while !cur.isEmpty do
      n += 1
      cur = cur.tail
    n

  def apply(i: Int): A =
    var cur: List[A] = this
    var k = i
    while k > 0 && !cur.isEmpty do
      cur = cur.tail
      k -= 1
    if cur.isEmpty || i < 0 then indexOutOfBounds(i.toString)
    else cur.head

  def headOption: Option[A] = if isEmpty then None else Some(head)

  def last: A =
    if isEmpty then noSuchElement("last of empty list")
    else
      var cur: List[A] = this
      while !cur.tail.isEmpty do cur = cur.tail
      cur.head

  def lastOption: Option[A] = if isEmpty then None else Some(last)
  override def init: List[A] = if isEmpty then unsupportedOperation("init of empty list") else dropRight(1)

  def toList: List[A] = this

  def foreach[U](f: A => U): Unit =
    var cur: List[A] = this
    while !cur.isEmpty do
      f(cur.head)
      cur = cur.tail

  def reverse: List[A] =
    var acc: List[A] = Nil
    var cur: List[A] = this
    while !cur.isEmpty do
      acc = cur.head :: acc
      cur = cur.tail
    acc

  def reverse_:::[B >: A](prefix: List[B]): List[B] =
    var acc: List[B] = this
    var cur: List[B] = prefix
    while !cur.isEmpty do
      acc = cur.head :: acc
      cur = cur.tail
    acc

  def map[B](f: A => B): List[B] =
    var acc: List[B] = Nil
    var cur: List[A] = this
    while !cur.isEmpty do
      acc = f(cur.head) :: acc
      cur = cur.tail
    acc.reverse

  // Each element mapped once, in order; the list itself when no result differs (`eq`) from
  // its element, else the results with the elements after the last change shared, as
  // scala-library's.
  final def mapConserve[B >: A <: AnyRef](f: A => B): List[B] =
    var results: List[B] = Nil
    var mapped = 0
    var kept: List[A] = this
    var keptAfter = 0
    var cur: List[A] = this
    while !cur.isEmpty do
      val x = cur.head
      val y = f(x)
      results = y :: results
      mapped += 1
      cur = cur.tail
      if !(y eq x.asInstanceOf[AnyRef]) then
        kept = cur
        keptAfter = mapped
    if keptAfter == 0 then this
    else
      var prefix = results.drop(mapped - keptAfter)
      var out: List[B] = kept
      while !prefix.isEmpty do
        out = prefix.head :: out
        prefix = prefix.tail
      out

  def flatMap[B](f: A => IterableOnce[B]): List[B] =
    var acc: List[B] = Nil
    var cur: List[A] = this
    while !cur.isEmpty do
      f(cur.head).foreach(x => acc = x :: acc)
      cur = cur.tail
    acc.reverse

  def filter(p: A => Boolean): List[A] =
    var acc: List[A] = Nil
    var cur: List[A] = this
    while !cur.isEmpty do
      if p(cur.head) then acc = cur.head :: acc
      cur = cur.tail
    acc.reverse

  def filterNot(p: A => Boolean): List[A] = filter(x => !p(x))

  def withFilter(p: A => Boolean): List[A] = filter(p)

  override def collectFirst[B](pf: PartialFunction[A, B]): Option[B] =
    var cur: List[A] = this
    var result: Option[B] = None
    val miss = partialMiss[A, B]
    while result.isEmpty && !cur.isEmpty do
      val v = pf.applyOrElse(cur.head, miss)
      if !isPartialMiss(v) then result = Some(v)
      cur = cur.tail
    result

  def foldLeft[B](z: B)(op: (B, A) => B): B =
    var acc = z
    var cur: List[A] = this
    while !cur.isEmpty do
      acc = op(acc, cur.head)
      cur = cur.tail
    acc

  def foldRight[B](z: B)(op: (A, B) => B): B =
    reverse.foldLeft(z)((acc, x) => op(x, acc))

  def reduce[B >: A](op: (B, B) => B): B =
    if isEmpty then unsupportedOperation("empty.reduceLeft")
    else tail.foldLeft[B](head)(op)

  def exists(p: A => Boolean): Boolean =
    var cur: List[A] = this
    var found = false
    while !found && !cur.isEmpty do
      found = p(cur.head)
      cur = cur.tail
    found

  def forall(p: A => Boolean): Boolean = !exists(x => !p(x))

  def contains[B >: A](elem: B): Boolean = exists(x => x == elem)

  def find(p: A => Boolean): Option[A] =
    var cur: List[A] = this
    var result: Option[A] = None
    while result.isEmpty && !cur.isEmpty do
      if p(cur.head) then result = Some(cur.head)
      cur = cur.tail
    result

  def count(p: A => Boolean): Int =
    var n = 0
    var cur: List[A] = this
    while !cur.isEmpty do
      if p(cur.head) then n += 1
      cur = cur.tail
    n

  def take(n: Int): List[A] =
    var acc: List[A] = Nil
    var cur: List[A] = this
    var k = n
    while k > 0 && !cur.isEmpty do
      acc = cur.head :: acc
      cur = cur.tail
      k -= 1
    acc.reverse

  def drop(n: Int): List[A] =
    var cur: List[A] = this
    var k = n
    while k > 0 && !cur.isEmpty do
      cur = cur.tail
      k -= 1
    cur

  def takeWhile(p: A => Boolean): List[A] =
    var acc: List[A] = Nil
    var cur: List[A] = this
    while !cur.isEmpty && p(cur.head) do
      acc = cur.head :: acc
      cur = cur.tail
    acc.reverse

  def dropWhile(p: A => Boolean): List[A] =
    var cur: List[A] = this
    while !cur.isEmpty && p(cur.head) do cur = cur.tail
    cur

  def :::[B >: A](prefix: List[B]): List[B] =
    var acc: List[B] = this
    var cur = prefix.reverse
    while !cur.isEmpty do
      acc = cur.head :: acc
      cur = cur.tail
    acc

  def ++[B >: A](suffix: IterableOnce[B]): List[B] = this ::: List.from(suffix)
  override def concat[B >: A](suffix: IterableOnce[B]): List[B] = this ::: List.from(suffix)
  override def prepended[B >: A](elem: B): List[B] = elem :: this
  override def prependedAll[B >: A](prefix: IterableOnce[B]): List[B] = List.from(prefix) ::: this
  def :+[B >: A](elem: B): List[B] = this ::: (elem :: Nil)

  def +:[B >: A](elem: B): List[B] = elem :: this

  def zipWithIndex: List[(A, Int)] =
    var acc: List[(A, Int)] = Nil
    var cur: List[A] = this
    var i = 0
    while !cur.isEmpty do
      acc = (cur.head, i) :: acc
      cur = cur.tail
      i += 1
    acc.reverse

  override def collectionClassName: String = "List"
  override def toString: String = mkString("List(", ", ", ")")

final case class ::[+A](head: A, tail: List[A]) extends List[A]:
  def isEmpty: Boolean = false

case object Nil extends List[Nothing]:
  def isEmpty: Boolean = true
  def head: Nothing = noSuchElement("head of empty list")
  def tail: List[Nothing] = unsupportedOperation("tail of empty list")

@js("$0.sort($1)")
def sortArray[A](arr: RawBuffer[A], compare: (A, A) => Int): Unit = scala.runtime.mergeSort(arr, compare)

def fromArray[A](arr: RawBuffer[A]): List[A] =
  var acc: List[A] = Nil
  var i = arr.length - 1
  while i >= 0 do
    acc = arr(i) :: acc
    i -= 1
  acc

object List:
  // scala-library's companion is an `IterableFactory`, whose `iterableFactory` a library body
  // names as the implicit `Factory` it resolved.
  implicit def iterableFactory[A]: Factory[A, List[A]] =
    new Factory[A, List[A]]:
      def fromSpecific(it: IterableOnce[A]): List[A] = from(it)
      def newBuilder: scala.collection.mutable.Builder[A, List[A]] = List.newBuilder[A]
  def apply[A](elems: A*): List[A] = elems.toList
  def empty[A]: List[A] = Nil
  def newBuilder[A]: scala.collection.mutable.Builder[A, List[A]] = scala.collection.mutable.ArrayBuffer.empty[A].mapResult(_.toList)
  def from[A](source: IterableOnce[A]): List[A] = source match
    case xs: List[?] => unsafeCast(xs)
    case _ => fromArray(iterableToArray(source))
  def concat[A](xss: IterableOnce[A]*): List[A] = xss.toList.flatMap(xs => xs)
  def iterate[A](start: A, len: Int)(f: A => A): List[A] = Iterator.iterate(start)(f).take(len).toList
  def unfold[A, S](init: S)(f: S => Option[(A, S)]): List[A] = Iterator.unfold(init)(f).toList
  def fill[A](n: Int)(elem: => A): List[A] =
    var acc: List[A] = Nil
    var i = 0
    while i < n do
      acc = elem :: acc
      i += 1
    acc
  def tabulate[A](n: Int)(f: Int => A): List[A] =
    var acc: List[A] = Nil
    var i = 0
    while i < n do
      acc = f(i) :: acc
      i += 1
    acc.reverse
  def range(start: Int, end: Int, step: Int = 1): List[Int] = Range(start, end, step).toList

// scala-library's `ArrayOps.WithFilter`: the elements that pass, each tested as the operation
// after it reaches it, so that a guard of a for comprehension runs interleaved with its body.
final class ArrayWithFilter[T](a: Array[T], p: T => Boolean):
  @jvmEvidence
  // `map` and `foreach` read each element once, before the guard runs, and pass that value on;
  // `flatMap` reads it again after the guard, as scala-library's does.
  def map[B: scala.reflect.ClassTag](f: T => B): Array[B] =
    val out = emptyBuffer[B]
    var i = 0
    while i < a.length do
      val x = a(i)
      if p(x) then out.push(f(x))
      i += 1
    taggedArray(out)
  @jvmEvidence
  def flatMap[B: scala.reflect.ClassTag](f: T => IterableOnce[B]): Array[B] =
    val out = emptyBuffer[B]
    var i = 0
    while i < a.length do
      if p(a(i)) then f(a(i)).iterator.foreach(y => out.push(y))
      i += 1
    taggedArray(out)
  def foreach[U](f: T => U): Unit =
    var i = 0
    while i < a.length do
      val x = a(i)
      if p(x) then f(x)
      i += 1
  def withFilter(q: T => Boolean): ArrayWithFilter[T] = new ArrayWithFilter(a, x => p(x) && q(x))

extension [T](a: Array[T])
  @js("$0.length")
  @jvm("$0 array_length")
  def size: Int
  def foreach[U](f: T => U): Unit =
    var i = 0
    while i < a.length do
      f(a(i))
      i += 1
  def toList: List[T] = fromArray(buffered(a))
  @jvmEvidence
  @js("$0.map((x) => $1(x))")
  def map[B: scala.reflect.ClassTag](f: T => B): Array[B] =
    val out = emptyBuffer[B]
    var i = 0
    while i < a.length do
      out.push(f(a(i)))
      i += 1
    taggedArray(out)
  @js("$0.filter((x) => $1(x))")
  def filter(p: T => Boolean): Array[T] =
    val out = emptyBuffer[T]
    var i = 0
    while i < a.length do
      if p(a(i)) then out.push(a(i))
      i += 1
    arrayLike(out, a)
  @jvmEvidence
  def collect[B: scala.reflect.ClassTag](pf: PartialFunction[T, B]): Array[B] = taggedArray(Vector.wrap(buffered(a)).collect(pf).unsafeArray)
  def collectFirst[B](pf: PartialFunction[T, B]): Option[B] = Vector.wrap(buffered(a)).collectFirst(pf)
  def find(p: T => Boolean): Option[T] =
    var i = 0
    var found: Option[T] = None
    while found.isEmpty && i < a.length do
      if p(a(i)) then found = Some(a(i))
      i += 1
    found
  def exists(p: T => Boolean): Boolean = a.find(p).isDefined
  def forall(p: T => Boolean): Boolean = !a.exists(x => !p(x))
  def contains(elem: T): Boolean = a.exists(_ == elem)
  def indexOf(elem: T): Int =
    var i = 0
    var at = -1
    while at < 0 && i < a.length do
      if a(i) == elem then at = i
      i += 1
    at
  def head: T = if a.length == 0 then noSuchElement("head of empty array") else a(0)
  def last: T = if a.length == 0 then noSuchElement("last of empty array") else a(a.length - 1)
  def headOption: Option[T] = if a.length == 0 then None else Some(a(0))
  def lastOption: Option[T] = if a.length == 0 then None else Some(a(a.length - 1))
  def foldLeft[B](z: B)(f: (B, T) => B): B =
    var acc = z
    var i = 0
    while i < a.length do
      acc = f(acc, a(i))
      i += 1
    acc
  def count(p: T => Boolean): Int = a.foldLeft(0)((n, x) => if p(x) then n + 1 else n)
  def toVector: Vector[T] = Vector.wrap(bufferedCopy(a))
  def toSeq: Seq[T] = new ArraySeq(a.clone())
  def toIndexedSeq: IndexedSeq[T] = new ArraySeq(a.clone())
  def toSet: Set[T] = Set.from(Vector.wrap(buffered(a)))
  def toBuffer: scala.collection.mutable.ArrayBuffer[T] = scala.collection.mutable.ArrayBuffer.from(Vector.wrap(buffered(a)))
  def toArray: Array[T] = a.clone()
  def iterator: Iterator[T] = arrayIterator(a)
  def view: View[T] = new View(() => arrayIterator(a), "View")
  def indices: Range = Range(0, a.length, 1)
  def mkString(start: String, sep: String, end: String): String = joinStrings(Vector.wrap(buffered(a)), start, sep, end)
  def mkString(sep: String): String = joinStrings(Vector.wrap(buffered(a)), "", sep, "")
  def mkString: String = joinStrings(Vector.wrap(buffered(a)), "", "", "")
  def isEmpty: Boolean = a.length == 0
  def nonEmpty: Boolean = a.length != 0
  def lengthIs: Int = a.length
  def sizeIs: Int = a.length
  def lengthCompare(len: Int): Int = if a.length < len then -1 else if a.length > len then 1 else 0
  def sizeCompare(len: Int): Int = a.lengthCompare(len)
  def tail: Array[T] = if a.length == 0 then unsupportedOperation("tail of empty array") else arrayLike(Vector.wrap(buffered(a)).tail.unsafeArray, a)
  def init: Array[T] = if a.length == 0 then unsupportedOperation("init of empty array") else arrayLike(Vector.wrap(buffered(a)).init.unsafeArray, a)
  def lift(i: Int): Option[T] = if i >= 0 && i < a.length then Some(a(i)) else None
  def filterNot(p: T => Boolean): Array[T] = a.filter(x => !p(x))
  def withFilter(p: T => Boolean): ArrayWithFilter[T] = new ArrayWithFilter(a, p)
  @jvmEvidence
  def flatMap[B: scala.reflect.ClassTag](f: T => IterableOnce[B]): Array[B] = taggedArray(Vector.wrap(buffered(a)).flatMap(f).unsafeArray)
  def zip[B](that: IterableOnce[B]): Array[(T, B)] = taggedArray(Vector.wrap(buffered(a)).zip(that).unsafeArray)
  def zipWithIndex: Array[(T, Int)] = taggedArray(Vector.wrap(buffered(a)).zipWithIndex.unsafeArray)
  def reverse: Array[T] = arrayLike(Vector.wrap(buffered(a)).reverse.unsafeArray, a)
  def sorted[B >: T](implicit ord: Ordering[B]): Array[T] = arrayLike(Vector.wrap(buffered(a)).sorted(using ord).unsafeArray, a)
  def sortBy[B](f: T => B)(implicit ord: Ordering[B]): Array[T] = arrayLike(Vector.wrap(buffered(a)).sortBy(f).unsafeArray, a)
  def sortWith(lt: (T, T) => Boolean): Array[T] = arrayLike(Vector.wrap(buffered(a)).sortWith(lt).unsafeArray, a)
  def distinct: Array[T] = arrayLike(Vector.wrap(buffered(a)).distinct.unsafeArray, a)
  def distinctBy[B](f: T => B): Array[T] = arrayLike(Vector.wrap(buffered(a)).distinctBy(f).unsafeArray, a)
  def take(n: Int): Array[T] = arrayLike(arraySlice(buffered(a), 0, Math.max(n, 0)), a)
  def drop(n: Int): Array[T] = arrayLike(arraySlice(buffered(a), Math.max(n, 0), a.length), a)
  def takeRight(n: Int): Array[T] = arrayLike(Vector.wrap(buffered(a)).takeRight(n).unsafeArray, a)
  def dropRight(n: Int): Array[T] = arrayLike(Vector.wrap(buffered(a)).dropRight(n).unsafeArray, a)
  def slice(from: Int, until: Int): Array[T] = arrayLike(arraySlice(buffered(a), Math.max(from, 0), Math.max(until, 0)), a)
  def takeWhile(p: T => Boolean): Array[T] = arrayLike(Vector.wrap(buffered(a)).takeWhile(p).unsafeArray, a)
  def dropWhile(p: T => Boolean): Array[T] = arrayLike(Vector.wrap(buffered(a)).dropWhile(p).unsafeArray, a)
  def partition(p: T => Boolean): (Array[T], Array[T]) = (a.filter(p), a.filterNot(p))
  def span(p: T => Boolean): (Array[T], Array[T]) = (a.takeWhile(p), a.dropWhile(p))
  def splitAt(n: Int): (Array[T], Array[T]) = (a.take(n), a.drop(n))
  def groupBy[K](f: T => K): Map[K, Array[T]] = Vector.wrap(buffered(a)).groupBy(f).mapValues(v => arrayLike(v.unsafeArray, a))
  def foldRight[B](z: B)(op: (T, B) => B): B = Vector.wrap(buffered(a)).foldRight(z)(op)
  def reduce[B >: T](op: (B, B) => B): B = Vector.wrap(buffered(a)).reduce(op)
  def sum[B >: T](implicit num: Numeric[B]): B = Vector.wrap(buffered(a)).sum(using num)
  def max[B >: T](implicit ord: Ordering[B]): T = Vector.wrap(buffered(a)).max(using ord)
  def min[B >: T](implicit ord: Ordering[B]): T = Vector.wrap(buffered(a)).min(using ord)
  def maxBy[B](f: T => B)(implicit ord: Ordering[B]): T = Vector.wrap(buffered(a)).maxBy(f)
  def minBy[B](f: T => B)(implicit ord: Ordering[B]): T = Vector.wrap(buffered(a)).minBy(f)
  def maxOption[B >: T](implicit ord: Ordering[B]): Option[T] = Vector.wrap(buffered(a)).maxOption(using ord)
  def minOption[B >: T](implicit ord: Ordering[B]): Option[T] = Vector.wrap(buffered(a)).minOption(using ord)
  def indexWhere(p: T => Boolean): Int = Vector.wrap(buffered(a)).indexWhere(p)
  def lastIndexWhere(p: T => Boolean, end: Int = Int.MaxValue): Int = Vector.wrap(buffered(a)).lastIndexWhere(p, end)
  // Inline, as `String`'s extensions of these names are not: a package-level extension is named
  // in the output by its place among the written ones of its name.
  inline def startsWith[B >: T](that: IterableOnce[B], offset: Int = 0): Boolean = Vector.wrap(buffered(a)).startsWith(that, offset)
  inline def sliding(size: Int, step: Int = 1): Iterator[Array[T]] = Vector.wrap(buffered(a)).sliding(size, step).map(w => arrayLike(w.unsafeArray, a))
  inline def grouped(size: Int): Iterator[Array[T]] = Vector.wrap(buffered(a)).grouped(size).map(g => arrayLike(g.unsafeArray, a))
  def findLast(p: T => Boolean): Option[T] = Vector.wrap(buffered(a)).findLast(p)
  def sameElements(that: IterableOnce[T]): Boolean = Vector.wrap(buffered(a)).sameElements(that)
  def updated(index: Int, elem: T): Array[T] = arrayLike(Vector.wrap(buffered(a)).updated(index, elem).unsafeArray, a)
  @jvmEvidence
  def appended[B >: T: scala.reflect.ClassTag](elem: B): Array[B] =
    val out = copyArray[T, B](buffered(a))
    out.push(elem)
    taggedArray(out)
  @jvmEvidence
  def prepended[B >: T: scala.reflect.ClassTag](elem: B): Array[B] = taggedArray(Vector.wrap(buffered(a)).prepended(elem).unsafeArray)
  @jvmEvidence
  def :+[B >: T: scala.reflect.ClassTag](elem: B): Array[B] = a.appended(elem)
  @jvmEvidence
  def +:[B >: T: scala.reflect.ClassTag](elem: B): Array[B] = a.prepended(elem)
  @jvmEvidence
  @js("$0.concat($1)")
  def ++[B >: T: scala.reflect.ClassTag](that: Array[B]): Array[B] =
    val out = copyArray[T, B](buffered(a))
    var i = 0
    while i < that.length do
      out.push(that(i))
      i += 1
    taggedArray(out)
  @jvmEvidence
  def ++[B >: T: scala.reflect.ClassTag](xs: IterableOnce[B]): Array[B] =
    val out = copyArray[T, B](buffered(a))
    xs.foreach(x => out.push(x))
    taggedArray(out)

extension [T](a: Array[T])
  def toMap[K, V](implicit ev: T <:< (K, V)): Map[K, V] = Map.from(Vector.wrap(buffered(unsafeCast[Array[T], Array[(K, V)]](a))))
  // `js.Array`'s own `slice`, which a Scala.js jar's body calls on a JS array, teq's `Array`.
  def jsSlice(start: Int, end: Int): Array[T] = arrayLike(arraySlice(buffered(a), start, end), a)

extension [K: scala.reflect.ClassTag, V: scala.reflect.ClassTag](a: Array[(K, V)])
  @jvmEvidence
  def unzip: (Array[K], Array[V]) = (a.map(p => p._1), a.map(p => p._2))

/** scala-library's `Predef.genericWrapArray`: an array where a collection is expected, wrapped
  * without a copy (`xs.joined` for an `extension (xs: IterableOnce[A])`). */
implicit def genericWrapArray[T](xs: Array[T]): ArraySeq[T] = ArraySeq.unsafeWrapArray(xs)

/** The array of `head` and of what `rest` holds. Where `rest` is written out, the compiler
  * puts the array literal of the elements in the place of the call. */
@jvmEvidence
inline def arrayOf[T: scala.reflect.ClassTag](inline head: T, inline rest: T*): Array[T] =
  val first = head
  val more = rest
  Array.apply[T](more.prepended(first)*)

object Array:
  @jvmEvidence
  def apply[T: scala.reflect.ClassTag](elems: T*): Array[T] = taggedArray(iterableToArray(elems))
  // scalac's overloads for the primitives, which take no tag and to whose element type the
  // arguments are converted (`Array(1, 2L)` is an `Array[Long]`).
  inline def apply(inline x: Boolean, inline xs: Boolean*): Array[Boolean] = arrayOf(x, xs*)
  inline def apply(inline x: Byte, inline xs: Byte*): Array[Byte] = arrayOf(x, xs*)
  inline def apply(inline x: Short, inline xs: Short*): Array[Short] = arrayOf(x, xs*)
  inline def apply(inline x: Char, inline xs: Char*): Array[Char] = arrayOf(x, xs*)
  inline def apply(inline x: Int, inline xs: Int*): Array[Int] = arrayOf(x, xs*)
  inline def apply(inline x: Long, inline xs: Long*): Array[Long] = arrayOf(x, xs*)
  inline def apply(inline x: Float, inline xs: Float*): Array[Float] = arrayOf(x, xs*)
  inline def apply(inline x: Double, inline xs: Double*): Array[Double] = arrayOf(x, xs*)
  inline def apply(inline x: Unit, inline xs: Unit*): Array[Unit] = arrayOf(x, xs*)
  def copyAs[T](original: Array[?], newLength: Int)(implicit ct: scala.reflect.ClassTag[T]): Array[T] =
    val out = ct.newArray(newLength)
    var i = 0
    while i < newLength && i < original.length do
      out(i) = unsafeCast(original(i))
      i += 1
    out
  @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
  def copyOf[T](original: Array[T], newLength: Int): Array[T] =
    val out = emptyBuffer[T]
    var i = 0
    while i < newLength do
      out.push(if i < original.length then original(i) else null.asInstanceOf[T])
      i += 1
    arrayLike(out, original)
  def ofDim[T](n: Int)(implicit ct: scala.reflect.ClassTag[T]): Array[T] = ct.newArray(n)
  @jvm("rt $1 rtcall taggedBuilder(Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  def newBuilder[T](implicit ct: scala.reflect.ClassTag[T]): scala.collection.mutable.ArrayBuilder[T] =
    new scala.collection.mutable.ArrayBuilder[T]
  def copy(src: AnyRef, srcPos: Int, dest: AnyRef, destPos: Int, length: Int): Unit =
    val from = unsafeCast[AnyRef, Array[Any]](src)
    val to = unsafeCast[AnyRef, Array[Any]](dest)
    var i = 0
    while i < length do
      to(destPos + i) = from(srcPos + i)
      i += 1
  implicit def toFactory[A](dummy: Array.type)(implicit ct: scala.reflect.ClassTag[A]): Factory[A, Array[A]] =
    new Factory[A, Array[A]]:
      def fromSpecific(it: IterableOnce[A]): Array[A] = taggedArray(iterableToArray(it))
      def newBuilder: scala.collection.mutable.Builder[A, Array[A]] = scala.collection.mutable.ArrayBuffer.empty[A].mapResult(_.toArray)
  @jvmEvidence
  @js("[]")
  @jvm("rt $1 iconst_0 rtcall newArrayOfTag(Lscala/reflect/ClassTag;I)Ljava/lang/Object; cast_result")
  def empty[T: scala.reflect.ClassTag]: Array[T]
  @jvmEvidence
  @js("$arrayFill($1, $2)")
  def fill[T: scala.reflect.ClassTag](n: Int)(elem: => T): Array[T] =
    val out = emptyBuffer[T]
    var i = 0
    while i < n do
      out.push(elem)
      i += 1
    taggedArray(out)
  @jvmEvidence
  @js("$arrayTabulate($1, $2)")
  def tabulate[T: scala.reflect.ClassTag](n: Int)(f: Int => T): Array[T] =
    val out = emptyBuffer[T]
    var i = 0
    while i < n do
      out.push(f(i))
      i += 1
    taggedArray(out)
  def range(start: Int, end: Int, step: Int = 1): Array[Int] = Range(start, end, step).toArray

final class Range(val start: Int, val end: Int, val step: Int, val isInclusive: Boolean = false) extends SeqOps[Int, IndexedSeq, IndexedSeq[Int]], IndexedSeq[Int]:
  override def collectionClassName: String = "Range"
  def buildCC[B](items: RawBuffer[B]): IndexedSeq[B] = Vector.wrap(items)
  def by(newStep: Int): Range = new Range(start, end, newStep, isInclusive)
  def inclusive: Range = new Range(start, end, step, true)
  // scala-library's arithmetic: the number of elements of a non-empty range, an unsigned `Int`
  // (0 for 2^32), and its last element, neither stepping past `Int.MaxValue`.
  private def count: Int =
    val stepSign = step >> 31
    val gap = ((end - start) ^ stepSign) - stepSign
    val absStep = (step ^ stepSign) - stepSign
    val div = if absStep == 1 then gap else java.lang.Integer.divideUnsigned(gap, absStep)
    if isInclusive || absStep * div != gap then div + 1 else div
  private def lastElement: Int =
    if step == 1 || step == -1 then (if isInclusive then end else end - step)
    else start + step * (count - 1)
  // Whether the non-negative `n` is at least the number of elements, compared unsigned.
  private def atLeastCount(n: Int): Boolean = (n ^ Int.MinValue) > ((count - 1) ^ Int.MinValue)
  private def emptyAt(at: Int): Range = new Range(at, at, step)
  override def isEmpty: Boolean =
    if isInclusive then (if step >= 0 then start > end else start < end)
    else if step >= 0 then start >= end
    else start <= end
  def length: Int =
    if isEmpty then 0
    else
      val n = count
      if n > 0 then n
      else throw new IllegalArgumentException(start.toString + (if isInclusive then " to " else " until ") + end.toString + " by " + step.toString + ": seqs cannot contain more than Int.MaxValue elements.")
  def apply(i: Int): Int =
    if i < 0 || i >= length then indexOutOfBounds(i.toString)
    else start + i * step
  override def head: Int = if isEmpty then noSuchElement("head on empty Range") else start
  override def last: Int = if isEmpty then noSuchElement("last on empty Range") else lastElement
  def foreach[U](f: Int => U): Unit =
    if !isEmpty then
      val last = lastElement
      var i = start
      var more = true
      while more do
        f(i)
        if i == last then more = false else i += step
  override def iterator: Iterator[Int] =
    var more = !isEmpty
    var i = start
    val last = lastElement
    def next(): Int =
      if !more then noSuchElement("next on empty iterator")
      val v = i
      if v == last then more = false else i += step
      v
    new FnIterator(() => more, () => next())
  override def toVector: Vector[Int] = Vector.tabulate(length)(i => start + i * step)
  override def toIndexedSeq: IndexedSeq[Int] = this
  // The arithmetic series under the standard `Numeric`, a custom one's `plus` per element.
  override def sum[B >: Int](implicit num: Numeric[B]): Int =
    if num.asInstanceOf[AnyRef] eq Numeric.IntIsIntegral then
      if isEmpty then 0
      else if length == 1 then head
      else ((length * (head.toLong + last)) / 2).toInt
    else if isEmpty then num.toInt(num.zero)
    else
      var acc = num.zero
      val last = lastElement
      var i = start
      var more = true
      while more do
        acc = num.plus(acc, i)
        if i == last then more = false else i += step
      num.toInt(acc)
  def contains(x: Int): Boolean =
    if isEmpty then false
    else if step > 0 then x >= start && x <= lastElement && (step == 1 || java.lang.Integer.remainderUnsigned(x - start, step) == 0)
    else x <= start && x >= lastElement && (step == -1 || java.lang.Integer.remainderUnsigned(start - x, -step) == 0)
  override def take(n: Int): Range =
    if n <= 0 || isEmpty then emptyAt(start)
    else if atLeastCount(n) then this
    else new Range(start, start + step * (n - 1), step, true)
  override def drop(n: Int): Range =
    if n <= 0 || isEmpty then this
    else if atLeastCount(n) then emptyAt(end)
    else new Range(start + step * n, end, step, isInclusive)
  override def slice(from: Int, until: Int): Range =
    if isEmpty then this
    else if from <= 0 then take(until)
    else if until >= 0 && atLeastCount(until) then drop(from)
    else if from >= until then emptyAt(start + step * from)
    else new Range(start + step * from, start + step * (until - 1), step, true)
  override def takeRight(n: Int): Range =
    if n <= 0 || isEmpty then emptyAt(start)
    else if atLeastCount(n) then this
    else new Range(start + step * (count - n), end, step, isInclusive)
  override def dropRight(n: Int): Range =
    if n <= 0 || isEmpty then this
    else if atLeastCount(n) then emptyAt(end)
    else new Range(start, start + step * (count - 1 - n), step, true)
  override def takeWhile(p: Int => Boolean): Range =
    val n = indexWhere(x => !p(x))
    if n < 0 then this else take(n)
  override def dropWhile(p: Int => Boolean): Range =
    val n = indexWhere(x => !p(x))
    if n < 0 then emptyAt(end) else drop(n)
  override def span(p: Int => Boolean): (Range, Range) = (takeWhile(p), dropWhile(p))
  override def splitAt(n: Int): (Range, Range) = (take(n), drop(n))
  override def tail: Range =
    if isEmpty then noSuchElement("tail on empty Range")
    else if count == 1 then emptyAt(end)
    else new Range(start + step, end, step, isInclusive)
  override def init: Range =
    if isEmpty then noSuchElement("init on empty Range") else dropRight(1)
  override def reverse: Range = if isEmpty then this else new Range(last, start, -step, true)
  override def toString: String =
    val inexact = if isInclusive then lastElement != end else lastElement + step != end
    val prefix = if isEmpty then "empty " else if inexact then "inexact " else ""
    prefix + "Range " + start.toString + (if isInclusive then " to " else " until ") + end.toString +
      (if step == 1 then "" else " by " + step.toString)

extension (x: Int)
  def until(end: Int): Range = Range(x, end, 1)
  def to(end: Int): Range = new Range(x, end, 1, true)
  def max(y: Int): Int = if x > y then x else y
  def min(y: Int): Int = if x < y then x else y
  def abs: Int = if x < 0 then -x else x
  def sign: Int = if x > 0 then 1 else if x < 0 then -1 else 0
  def signum: Int = sign
  def compare(y: Int): Int = comparePrimitives(x, y)
  @js("($0 >>> 0).toString(16)")
  @jvm("invokestatic java/lang/Integer.toHexString(I)Ljava/lang/String;")
  def toHexString: String
  @js("($0 >>> 0).toString(8)")
  @jvm("invokestatic java/lang/Integer.toOctalString(I)Ljava/lang/String;")
  def toOctalString: String
  @js("($0 >>> 0).toString(2)")
  @jvm("invokestatic java/lang/Integer.toBinaryString(I)Ljava/lang/String;")
  def toBinaryString: String

extension (c: Char)
  def to(end: Char): NumericRange[Char] = new NumericRange(c.toLong, end.toLong, 1L, true, i => i.toInt.toChar)
  def until(end: Char): NumericRange[Char] = new NumericRange(c.toLong, end.toLong, 1L, false, i => i.toInt.toChar)
  def compare(that: Char): Int = c.toInt - that.toInt
  def max(that: Char): Char = if c.toInt >= that.toInt then c else that
  def min(that: Char): Char = if c.toInt <= that.toInt then c else that

// The ranges of Long and Char: the bounds are Longs that `elem` turns into elements.
final class NumericRange[A](val startIndex: Long, val endIndex: Long, val step: Long, val isInclusive: Boolean, elem: Long => A) extends SeqOps[A, IndexedSeq, IndexedSeq[A]], IndexedSeq[A]:
  def buildCC[B](items: RawBuffer[B]): IndexedSeq[B] = Vector.wrap(items)
  def start: A = elem(startIndex)
  def end: A = elem(endIndex)
  def by(newStep: Long): NumericRange[A] = new NumericRange(startIndex, endIndex, newStep, isInclusive, elem)
  def inclusive: NumericRange[A] = new NumericRange(startIndex, endIndex, step, true, elem)
  private def limit: Long = if !isInclusive then endIndex else if step > 0L then endIndex + 1L else endIndex - 1L
  override def isEmpty: Boolean = if step > 0L then startIndex >= limit else startIndex <= limit
  def length: Int =
    if isEmpty then 0
    else if step > 0L then ((limit - startIndex + step - 1L) / step).toInt
    else ((startIndex - limit - step - 1L) / (-step)).toInt
  def apply(i: Int): A =
    if i < 0 || i >= length then indexOutOfBounds(i.toString)
    else elem(startIndex + i.toLong * step)
  def foreach[U](f: A => U): Unit =
    var i = startIndex
    val stop = limit
    if step > 0L then
      while i < stop do
        f(elem(i))
        i += step
    else
      while i > stop do
        f(elem(i))
        i += step
  override def iterator: Iterator[A] =
    var i = startIndex
    val stop = limit
    def next(): A =
      i += step
      elem(i - step)
    new FnIterator(() => if step > 0L then i < stop else i > stop, () => next())
  override def toString: String =
    (if isEmpty then "empty " else "") + "NumericRange " + start.toString + (if isInclusive then " to " else " until ") +
      end.toString + (if step == 1L then "" else " by " + elem(step).toString)

extension (x: Long)
  def until(end: Long): NumericRange[Long] = new NumericRange(x, end, 1L, false, i => i)
  def to(end: Long): NumericRange[Long] = new NumericRange(x, end, 1L, true, i => i)
  def max(y: Long): Long = if x > y then x else y
  def min(y: Long): Long = if x < y then x else y
  def abs: Long = if x < 0L then -x else x
  def sign: Long = if x > 0L then 1L else if x < 0L then -1L else 0L
  def signum: Int = if x > 0L then 1 else if x < 0L then -1 else 0
  def compare(y: Long): Int = comparePrimitives(x, y)
  @js("BigInt.asUintN(64, $0).toString(16)")
  @jvm("invokestatic java/lang/Long.toHexString(J)Ljava/lang/String;")
  def toHexString: String
  @js("BigInt.asUintN(64, $0).toString(8)")
  @jvm("invokestatic java/lang/Long.toOctalString(J)Ljava/lang/String;")
  def toOctalString: String
  @js("BigInt.asUintN(64, $0).toString(2)")
  @jvm("invokestatic java/lang/Long.toBinaryString(J)Ljava/lang/String;")
  def toBinaryString: String

extension (x: Double)
  // `Math.max` and `Math.min` of Java: a NaN wins, and 0.0 is above -0.0.
  def max(y: Double): Double =
    if x != x then x else if x == 0.0 && y == 0.0 then (if 1.0 / x < 0.0 then y else x) else if x >= y then x else y
  def compare(y: Double): Int = compareDoubles(x, y)
  def min(y: Double): Double =
    if x != x then x else if x == 0.0 && y == 0.0 then (if 1.0 / x < 0.0 then x else y) else if x <= y then x else y
  def abs: Double = if x <= 0.0 then 0.0 - x else x
  @js("$d2l(Math.round($0))")
  @jvm("invokestatic java/lang/Math.round(D)J")
  def round: Long
  @js("$signum($0)")
  @jvm("invokestatic java/lang/Math.signum(D)D")
  def sign: Double
  @js("$signum($0)")
  @jvm("$0 invokestatic java/lang/Math.signum(D)D d2i")
  def signum: Int
  @js("Number.isFinite($0)")
  @jvm("invokestatic java/lang/Double.isFinite(D)Z")
  def isFinite: Boolean
  @js("($0 === Infinity || $0 === -Infinity)")
  @jvm("invokestatic java/lang/Double.isInfinite(D)Z")
  def isInfinite: Boolean
  def isInfinity: Boolean = x.isInfinite
  @js("($0 === Math.trunc($0) && Number.isFinite($0))")
  def isWhole: Boolean = !x.isInfinite && !x.isNaN && x == x.floor
  @js("($0 * 180 / Math.PI)")
  @jvm("invokestatic java/lang/Math.toDegrees(D)D")
  def toDegrees: Double
  @js("($0 / 180 * Math.PI)")
  @jvm("invokestatic java/lang/Math.toRadians(D)D")
  def toRadians: Double
  @js("Math.floor($0)")
  @jvm("invokestatic java/lang/Math.floor(D)D")
  def floor: Double
  @js("Math.ceil($0)")
  @jvm("invokestatic java/lang/Math.ceil(D)D")
  def ceil: Double
  @js("Number.isNaN($0)")
  @jvm("invokestatic java/lang/Double.isNaN(D)Z")
  def isNaN: Boolean

/** The java.lang.Math surface that Scala code reaches as `Math`. */
object Math:
  val PI: Double = 3.141592653589793
  val E: Double = 2.718281828459045
  @js("$max($1, $2)")
  @jvm("rt $1:L $2:L rtcall numMax(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
  def max[T <: Int | Long | Double](a: T, b: T): T
  @js("$min($1, $2)")
  @jvm("rt $1:L $2:L rtcall numMin(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
  def min[T <: Int | Long | Double](a: T, b: T): T
  @js("$abs($1)")
  @jvm("rt $1:L rtcall numAbs(Ljava/lang/Object;)Ljava/lang/Object;")
  def abs[T <: Int | Long | Double](x: T): T
  @js("Math.sqrt($1)")
  @jvm("invokestatic java/lang/Math.sqrt(D)D")
  def sqrt(x: Double): Double
  @js("Math.pow($1, $2)")
  @jvm("invokestatic java/lang/Math.pow(DD)D")
  def pow(x: Double, y: Double): Double
  @js("Math.floor($1)")
  @jvm("invokestatic java/lang/Math.floor(D)D")
  def floor(x: Double): Double
  @js("Math.ceil($1)")
  @jvm("invokestatic java/lang/Math.ceil(D)D")
  def ceil(x: Double): Double
  @js("$d2l(Math.round($1))")
  @jvm("invokestatic java/lang/Math.round(D)J")
  def round(x: Double): Long
  @js("Math.log($1)")
  @jvm("invokestatic java/lang/Math.log(D)D")
  def log(x: Double): Double
  @js("Math.log10($1)")
  @jvm("invokestatic java/lang/Math.log10(D)D")
  def log10(x: Double): Double
  @js("Math.exp($1)")
  @jvm("invokestatic java/lang/Math.exp(D)D")
  def exp(x: Double): Double
  @js("Math.sin($1)")
  @jvm("invokestatic java/lang/Math.sin(D)D")
  def sin(x: Double): Double
  @js("Math.cos($1)")
  @jvm("invokestatic java/lang/Math.cos(D)D")
  def cos(x: Double): Double
  @js("Math.tan($1)")
  @jvm("invokestatic java/lang/Math.tan(D)D")
  def tan(x: Double): Double
  @js("Math.atan2($1, $2)")
  @jvm("invokestatic java/lang/Math.atan2(DD)D")
  def atan2(y: Double, x: Double): Double
  @js("Math.hypot($1, $2)")
  @jvm("invokestatic java/lang/Math.hypot(DD)D")
  def hypot(x: Double, y: Double): Double
  @js("Math.cbrt($1)")
  @jvm("invokestatic java/lang/Math.cbrt(D)D")
  def cbrt(x: Double): Double
  @javaDefined @js("Math.random()")
  @jvm("invokestatic java/lang/Math.random()D")
  def random(): Double
  @js("$signum($1)")
  @jvm("invokestatic java/lang/Math.signum(D)D")
  def signum(x: Double): Double
  @js("$floorDiv($1, $2)")
  @jvm("rt $1:L $2:L rtcall numFloorDiv(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
  def floorDiv[T <: Int | Long](a: T, b: T): T
  @js("$floorMod($1, $2)")
  @jvm("rt $1:L $2:L rtcall numFloorMod(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")
  def floorMod[T <: Int | Long](a: T, b: T): T
  @js("$rint($1)")
  @jvm("invokestatic java/lang/Math.rint(D)D")
  def rint(x: Double): Double

@js("$parseIntRadix($0, $1)")
@jvm("invokestatic java/lang/Integer.parseInt(Ljava/lang/String;I)I")
def parseIntRadix(s: String, radix: Int): Int

/** The static side of java.lang.String. */
@javaDefined
object String:
  def valueOf(x: Any): String = x.toString
  def format(fmt: String, args: Any*): String = fmt.formatImpl(taggedArray(iterableToArray(args)))

object Int:
  val MaxValue: Int = 2147483647
  val MinValue: Int = -2147483648
  // scalac's widenings, which a library body names when it applies one explicitly; the typer
  // widens by itself, so they take no part in a conversion search.
  def int2long(x: Int): Long = x.toLong
  def int2float(x: Int): Float = x.toFloat
  def int2double(x: Int): Double = x.toDouble

object Long:
  val MaxValue: Long = 9223372036854775807L
  val MinValue: Long = -9223372036854775807L - 1L
  def long2float(x: Long): Float = x.toFloat
  def long2double(x: Long): Double = x.toDouble

object Char:
  val MaxValue: Char = '\uffff'
  val MinValue: Char = '\u0000'
  def char2int(x: Char): Int = x.toInt
  def char2long(x: Char): Long = x.toLong
  def char2float(x: Char): Float = x.toFloat
  def char2double(x: Char): Double = x.toDouble

object Byte:
  val MaxValue: Byte = 127
  val MinValue: Byte = -128
  def byte2short(x: Byte): Short = x.toShort
  def byte2int(x: Byte): Int = x.toInt
  def byte2long(x: Byte): Long = x.toLong
  def byte2float(x: Byte): Float = x.toFloat
  def byte2double(x: Byte): Double = x.toDouble
  given ByteOrdering: Ordering[Byte] = Ordering.Byte
  given ByteIsIntegral: Integral[Byte] with
    def zero: Byte = 0
    def one: Byte = 1
    def plus(a: Byte, b: Byte): Byte = (a + b).toByte
    def minus(a: Byte, b: Byte): Byte = (a - b).toByte
    def times(a: Byte, b: Byte): Byte = (a * b).toByte
    def quot(a: Byte, b: Byte): Byte = (a / b).toByte
    def rem(a: Byte, b: Byte): Byte = (a % b).toByte
    def fromInt(x: Int): Byte = x.toByte
    def toInt(x: Byte): Int = x.toInt
    def toLong(x: Byte): Long = x.toLong
    def toDouble(x: Byte): Double = x.toDouble
    def compare(a: Byte, b: Byte): Int = comparePrimitives(a, b)

object Short:
  val MaxValue: Short = 32767
  val MinValue: Short = -32768
  def short2int(x: Short): Int = x.toInt
  def short2long(x: Short): Long = x.toLong
  def short2float(x: Short): Float = x.toFloat
  def short2double(x: Short): Double = x.toDouble
  given ShortOrdering: Ordering[Short] = Ordering.Short
  given ShortIsIntegral: Integral[Short] with
    def zero: Short = 0
    def one: Short = 1
    def plus(a: Short, b: Short): Short = (a + b).toShort
    def minus(a: Short, b: Short): Short = (a - b).toShort
    def times(a: Short, b: Short): Short = (a * b).toShort
    def quot(a: Short, b: Short): Short = (a / b).toShort
    def rem(a: Short, b: Short): Short = (a % b).toShort
    def fromInt(x: Int): Short = x.toShort
    def toInt(x: Short): Int = x.toInt
    def toLong(x: Short): Long = x.toLong
    def toDouble(x: Short): Double = x.toDouble
    def compare(a: Short, b: Short): Int = comparePrimitives(a, b)

object Float:
  val MaxValue: Float = 3.4028235e38f
  val MinValue: Float = -3.4028235e38f
  val MinPositiveValue: Float = 1.4e-45f
  @js("Infinity")
  @jvm("getstatic java/lang/Float.POSITIVE_INFINITY:F")
  def PositiveInfinity: Float
  @js("(-Infinity)")
  @jvm("getstatic java/lang/Float.NEGATIVE_INFINITY:F")
  def NegativeInfinity: Float
  @js("NaN")
  @jvm("getstatic java/lang/Float.NaN:F")
  def NaN: Float
  def float2double(x: Float): Double = x.toDouble
  given FloatOrdering: Ordering[Float] = Ordering.DeprecatedFloatOrdering
  given FloatIsFractional: Fractional[Float] with
    def zero: Float = 0.0f
    def one: Float = 1.0f
    def plus(a: Float, b: Float): Float = a + b
    def minus(a: Float, b: Float): Float = a - b
    def times(a: Float, b: Float): Float = a * b
    def div(a: Float, b: Float): Float = a / b
    def fromInt(x: Int): Float = x.toFloat
    def toInt(x: Float): Int = x.toInt
    def toLong(x: Float): Long = x.toLong
    def toDouble(x: Float): Double = x.toDouble
    def compare(a: Float, b: Float): Int = compareDoubles(a, b)

object Double:
  val MaxValue: Double = 1.7976931348623157e308
  val MinValue: Double = -1.7976931348623157e308
  @js("Infinity")
  @jvm("getstatic java/lang/Double.POSITIVE_INFINITY:D")
  def PositiveInfinity: Double
  @js("(-Infinity)")
  @jvm("getstatic java/lang/Double.NEGATIVE_INFINITY:D")
  def NegativeInfinity: Double
  @js("NaN")
  @jvm("getstatic java/lang/Double.NaN:D")
  def NaN: Double
  val MinPositiveValue: Double = 4.9e-324
