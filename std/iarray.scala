package scala

import scala.collection.immutable.ArraySeq

// An immutable view of an array: the array itself, with the members that do not write to it,
// and `ArraySeq` for the rest of the sequence API.
opaque type IArray[+T] = Array[T]

object IArray:
  def unsafeFromArray[T](s: Array[T]): IArray[T] = s
  @jvmEvidence
  def apply[T: scala.reflect.ClassTag](xs: T*): IArray[T] = taggedArray(iterableToArray(xs))
  @jvmEvidence
  def empty[T: scala.reflect.ClassTag]: IArray[T] = taggedArray(emptyArray[T])
  @jvmEvidence
  def from[T: scala.reflect.ClassTag](source: IterableOnce[T]): IArray[T] = taggedArray(iterableToArray(source))
  @jvmEvidence
  def fill[T: scala.reflect.ClassTag](n: Int)(elem: => T): IArray[T] = tabulate(n)(_ => elem)
  @jvmEvidence
  def tabulate[T: scala.reflect.ClassTag](n: Int)(f: Int => T): IArray[T] =
    val out = emptyArray[T]
    var i = 0
    while i < n do
      out.push(f(i))
      i += 1
    taggedArray(out)
  implicit def genericWrapArray[T](arr: IArray[T]): ArraySeq[T] = ArraySeq.unsafeWrapArray(arr)
  implicit def wrapRefArray[T <: AnyRef](arr: IArray[T]): ArraySeq[T] = ArraySeq.unsafeWrapArray(arr)

  // The members loop over the array itself or go through a list: under `--std=scala-library`
  // the array's own collection API is scala-library's `ArrayOps`, whose transforms ask for a
  // `ClassTag`, and inside this object an `Array[T]` receiver would find these very methods.
  extension [T](arr: IArray[T])
    def length: Int = (arr: Array[T]).length
    def size: Int = length
    def apply(i: Int): T = (arr: Array[T])(i)
    def isEmpty: Boolean = length == 0
    def nonEmpty: Boolean = length != 0
    // Thrown by the class, this file being compiled under scala-library too, where the lean
    // prelude's helpers are not in scope.
    def head: T = if isEmpty then throw new java.util.NoSuchElementException("head of empty array") else apply(0)
    def headOption: Option[T] = if isEmpty then None else Some(head)
    def last: T = if isEmpty then throw new java.util.NoSuchElementException("last of empty array") else apply(length - 1)
    def lastOption: Option[T] = if isEmpty then None else Some(last)
    def indices: Range = 0 until length
    def iterator: Iterator[T] = toList.iterator
    def toArray: Array[T] = (arr: Array[T]).clone()
    def toList: List[T] =
      var out: List[T] = Nil
      var i = length - 1
      while i >= 0 do
        out = apply(i) :: out
        i -= 1
      out
    def toSeq: Seq[T] = ArraySeq.unsafeWrapArray(arr)
    def toIndexedSeq: IndexedSeq[T] = ArraySeq.unsafeWrapArray(arr)
    def toVector: Vector[T] = toList.toVector
    def toSet: Set[T] = toList.toSet
    @jvmEvidence
    def map[U: scala.reflect.ClassTag](f: T => U): IArray[U] =
      val out = emptyArray[U]
      var i = 0
      while i < length do
        out.push(f(apply(i)))
        i += 1
      taggedArray(out)
    @jvmEvidence
    def flatMap[U: scala.reflect.ClassTag](f: T => IterableOnce[U]): IArray[U] = taggedArray(iterableToArray(toList.flatMap(f)))
    def filter(p: T => Boolean): IArray[T] =
      val out = emptyArray[T]
      var i = 0
      while i < length do
        if p(apply(i)) then out.push(apply(i))
        i += 1
      arrayLike(out, arr)
    def filterNot(p: T => Boolean): IArray[T] = filter(x => !p(x))
    def find(p: T => Boolean): Option[T] =
      var i = 0
      while i < length do
        if p(apply(i)) then return Some(apply(i))
        i += 1
      None
    def collectFirst[U](f: PartialFunction[T, U]): Option[U] =
      var i = 0
      while i < length do
        if f.isDefinedAt(apply(i)) then return Some(f(apply(i)))
        i += 1
      None
    def exists(p: T => Boolean): Boolean = find(p).isDefined
    def forall(p: T => Boolean): Boolean = !exists(x => !p(x))
    def count(p: T => Boolean): Int =
      var n = 0
      var i = 0
      while i < length do
        if p(apply(i)) then n += 1
        i += 1
      n
    def foreach[U](f: T => U): Unit =
      var i = 0
      while i < length do
        f(apply(i))
        i += 1
    def contains(elem: T): Boolean = indexOf(elem) >= 0
    def indexOf(elem: T): Int = indexWhere(_ == elem)
    def indexWhere(p: T => Boolean): Int =
      var i = 0
      while i < length do
        if p(apply(i)) then return i
        i += 1
      -1
    def lift(i: Int): Option[T] = if i >= 0 && i < length then Some(apply(i)) else None
    def slice(from: Int, until: Int): IArray[T] =
      val out = emptyArray[T]
      var i = if from < 0 then 0 else from
      while i < until && i < length do
        out.push(apply(i))
        i += 1
      arrayLike(out, arr)
    def take(n: Int): IArray[T] = slice(0, n)
    def drop(n: Int): IArray[T] = slice(n, length)
    def takeWhile(p: T => Boolean): IArray[T] =
      val stop = indexWhere(x => !p(x))
      if stop < 0 then arr else slice(0, stop)
    def dropWhile(p: T => Boolean): IArray[T] =
      val start = indexWhere(x => !p(x))
      if start < 0 then IArray.empty[T] else slice(start, length)
    def reverse: IArray[T] =
      val out = emptyArray[T]
      var i = length - 1
      while i >= 0 do
        out.push(apply(i))
        i -= 1
      arrayLike(out, arr)
    def zip[U](that: IArray[U]): IArray[(T, U)] =
      val out = buffered(new Array[(T, U)](0))
      var i = 0
      while i < length && i < that.length do
        out.push((apply(i), that(i)))
        i += 1
      taggedArray(out)
    def zipWithIndex: IArray[(T, Int)] =
      val out = buffered(new Array[(T, Int)](0))
      var i = 0
      while i < length do
        out.push((apply(i), i))
        i += 1
      taggedArray(out)
    def foldLeft[B](z: B)(op: (B, T) => B): B =
      var acc = z
      var i = 0
      while i < length do
        acc = op(acc, apply(i))
        i += 1
      acc
    def foldRight[B](z: B)(op: (T, B) => B): B =
      var acc = z
      var i = length - 1
      while i >= 0 do
        acc = op(apply(i), acc)
        i -= 1
      acc
    def mkString: String = mkString("", "", "")
    def mkString(sep: String): String = mkString("", sep, "")
    def mkString(start: String, sep: String, end: String): String =
      var out = start
      var i = 0
      while i < length do
        if i > 0 then out = out + sep
        out = out + apply(i)
        i += 1
      out + end
    def ++(that: IArray[T]): IArray[T] =
      val out = bufferedCopy(arr: Array[T])
      that.foreach(x => out.push(x))
      arrayLike(out, arr)
    def appended(x: T): IArray[T] =
      val out = bufferedCopy(arr: Array[T])
      out.push(x)
      arrayLike(out, arr)
    def prepended(x: T): IArray[T] =
      val out = emptyArray[T]
      out.push(x)
      foreach(y => out.push(y))
      arrayLike(out, arr)
    def sorted[B >: T](implicit ord: Ordering[B]): IArray[T] = arrayLike(iterableToArray(toList.sorted(using ord)), arr)
    def sortBy[B](f: T => B)(implicit ord: Ordering[B]): IArray[T] = arrayLike(iterableToArray(toList.sortBy(f)(using ord)), arr)
    def distinct: IArray[T] = arrayLike(iterableToArray(toList.distinct), arr)
    def sum[B >: T](implicit num: Numeric[B]): B = toList.sum(using num)
    def max[B >: T](implicit ord: Ordering[B]): T = toList.max(using ord)
    def min[B >: T](implicit ord: Ordering[B]): T = toList.min(using ord)
    def corresponds[U](that: IterableOnce[U])(p: (T, U) => Boolean): Boolean = toList.corresponds(that)(p)
