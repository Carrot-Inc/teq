// The objects of scala-library's `scala.runtime` that compiled library bodies call: the case
// class helpers, the failure of an inlined `assert`, the equality of boxed numbers.
package scala.runtime

object ScalaRunTime:
  def _hashCode(x: Product): Int = scala.util.hashing.MurmurHash3.productHash(x)
  def _toString(x: Product): String = x.productIterator.mkString(x.productPrefix + "(", ",", ")")
  def hash(x: Any): Int = x.##
  def isArray(x: Any): Boolean = x.isInstanceOf[Array[?]]
  // scala-library's: `f` of a non-null `a` (its `Predef.wrapString` and the other wrappers).
  private[scala] inline def mapNull[A, B](a: A, inline f: B): B =
    if (a: A | Null) == null then null.asInstanceOf[B] else f

object Scala3RunTime:
  def assertFailed(message: Any): Nothing = throw new AssertionError("assertion failed: " + message)
  def assertFailed(): Nothing = throw new AssertionError("assertion failed")
  def nnFail(): Nothing = throw new NullPointerException("tried to cast away nullability, but value is null")

object BoxesRunTime:
  def equals(x: Any, y: Any): Boolean = x == y
  def equals2(x: Any, y: Any): Boolean = x == y
  def equalsNumObject(x: java.lang.Number, y: Any): Boolean = (x: Any) == y
  def equalsNumNum(x: java.lang.Number, y: java.lang.Number): Boolean = (x: Any) == (y: Any)
  def equalsCharObject(x: java.lang.Character, y: Any): Boolean = (x: Any) == y

// The base classes Scala 2's case class companions extend (`object Die extends AbstractFunction2[..]`).
abstract class AbstractFunction0[+R] extends (() => R)
abstract class AbstractFunction1[-T1, +R] extends (T1 => R)
abstract class AbstractFunction2[-T1, -T2, +R] extends ((T1, T2) => R)
abstract class AbstractFunction3[-T1, -T2, -T3, +R] extends ((T1, T2, T3) => R)
abstract class AbstractFunction4[-T1, -T2, -T3, -T4, +R] extends ((T1, T2, T3, T4) => R)
abstract class AbstractFunction5[-T1, -T2, -T3, -T4, -T5, +R] extends ((T1, T2, T3, T4, T5) => R)
abstract class AbstractFunction6[-T1, -T2, -T3, -T4, -T5, -T6, +R] extends ((T1, T2, T3, T4, T5, T6) => R)
abstract class AbstractFunction7[-T1, -T2, -T3, -T4, -T5, -T6, -T7, +R] extends ((T1, T2, T3, T4, T5, T6, T7) => R)
abstract class AbstractFunction8[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8) => R)
abstract class AbstractFunction9[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9) => R)
abstract class AbstractFunction10[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) => R)
abstract class AbstractFunction11[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11) => R)
abstract class AbstractFunction12[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12) => R)
abstract class AbstractFunction13[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13) => R)
abstract class AbstractFunction14[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14) => R)
abstract class AbstractFunction15[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, -T15, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15) => R)
abstract class AbstractFunction16[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, -T15, -T16, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16) => R)
abstract class AbstractFunction17[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, -T15, -T16, -T17, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17) => R)
abstract class AbstractFunction18[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, -T15, -T16, -T17, -T18, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18) => R)
abstract class AbstractFunction19[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, -T15, -T16, -T17, -T18, -T19, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19) => R)
abstract class AbstractFunction20[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, -T15, -T16, -T17, -T18, -T19, -T20, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20) => R)
abstract class AbstractFunction21[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, -T15, -T16, -T17, -T18, -T19, -T20, -T21, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21) => R)
abstract class AbstractFunction22[-T1, -T2, -T3, -T4, -T5, -T6, -T7, -T8, -T9, -T10, -T11, -T12, -T13, -T14, -T15, -T16, -T17, -T18, -T19, -T20, -T21, -T22, +R] extends ((T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22) => R)

// scalac's tuple of more than 22 elements: an immutable array of them, which `Tuple.fromArray`,
// `Tuple.fromIArray` and the generic operations below make past `Tuple22`. It is a `*:` chain's
// value at run time (a type test of `NonEmptyTuple` or `h *: t` takes it) without being one of
// the sealed trait's cases. Equality, hash and text are scalac's: equal elements by `==` at one
// arity, the product hash with the prefix "Tuple", the elements in parentheses.
final class TupleXXL private (es: IArray[Object]) extends Product with NonEmptyTuple:
  assert(unsafeCast[IArray[Object], Array[Object]](es).length > 22)

  private inline def arr: Array[Object] = unsafeCast(es)

  def productElement(n: Int): Any =
    if n < 0 || n >= arr.length then outOfBounds("Index " + n + " out of bounds for length " + arr.length)
    arr(n)
  def productArity: Int = arr.length
  override def productPrefix: String = "Tuple"

  override def toString: String =
    var s = "("
    var i = 0
    while i < arr.length do
      if i > 0 then s += ","
      s += arr(i)
      i += 1
    s + ")"

  override def hashCode: Int =
    var h = mixHash(0xcafebabe, 81172392) // MurmurHash3.productSeed, "Tuple".hashCode
    var i = 0
    while i < arr.length do
      h = mixHash(h, arr(i).##)
      i += 1
    finalizeHash(h, arr.length)

  override def canEqual(that: Any): Boolean = that match
    case that: TupleXXL => that.productArity == this.productArity
    case _ => false

  override def equals(that: Any): Boolean = that match
    case that: TupleXXL =>
      val other = that.arr
      (arr eq other) || arr.length == other.length && {
        var i = 0
        while i < arr.length && arr(i) == other(i) do i += 1
        i == arr.length
      }
    case _ => false

  def elems: IArray[Object] = es

  def tailXXL: TupleXXL =
    assert(arr.length > 23)
    new TupleXXL(unsafeCast(Tuples.slice(arr, 1, arr.length)))

  def toArray: Array[Object] = arr.clone()

object TupleXXL:
  def fromIterator(elems: Iterator[Any]): TupleXXL =
    val out = emptyArray[Object]
    elems.foreach(x => out.push(x.asInstanceOf[Object]))
    new TupleXXL(unsafeCast(untaggedArray(out)))
  def fromIArray(elems: IArray[Object]): TupleXXL = new TupleXXL(elems)
  // `Tuple.fromArray` past 22 elements: a copy, as scalac's, which a later write to `xs` leaves alone.
  private[scala] def copyOf[T](xs: Array[T]): TupleXXL =
    val arr = new Array[Object](xs.length)
    var i = 0
    while i < arr.length do
      arr(i) = xs(i).asInstanceOf[Object]
      i += 1
    new TupleXXL(unsafeCast(arr))
  def apply(elems: Any*): TupleXXL = fromIterator(elems.iterator)
  def unapplySeq(x: TupleXXL): Option[Seq[Any]] = Some(scala.collection.immutable.ArraySeq.unsafeWrapArray(x.toArray))

// TupleXXL's failure and hash steps in JavaScript are the runtime's own, which keeps the
// exception classes and the MurmurHash3 object out of a bundle that only makes such tuples.
@js("$fail(\"ArrayIndexOutOfBoundsException\", $0)")
private def outOfBounds(message: String): Nothing = throw new ArrayIndexOutOfBoundsException(message)
@js("$mix($0, $1)")
private def mixHash(hash: Int, data: Int): Int = scala.util.hashing.MurmurHash3.mix(hash, data)
@js("$finalizeHash($0, $1)")
private def finalizeHash(hash: Int, length: Int): Int = scala.util.hashing.MurmurHash3.finalizeHash(hash, length)

// What scalac's inline members of `Tuple` call on a tuple whose length the type does not give
// (`Tuple.fromArray(xs).size`, `t ++ u` of two `Tuple`s), and what library bodies compiled
// against scala-library call by name. Each reads the elements as an array of references and
// gives the result back through `Tuple.fromIArray`, so that up to 22 elements are `Tuple1` to
// `Tuple22` and more a `TupleXXL`, as scalac's runtime has them. A tuple whose type is known
// never comes here: the compiler reads and builds its fields itself.
object Tuples:
  inline val MaxSpecialized = 22

  def toArray(self: Tuple): Array[Object] = (self: Any) match
    case self: TupleXXL => self.toArray
    case _ => productToArray(self.asInstanceOf[Product])

  def toIArray(self: Tuple): IArray[Object] = unsafeCast(elements(self))

  def productToArray(self: Product): Array[Object] =
    val arr = new Array[Object](self.productArity)
    var i = 0
    while i < arr.length do
      arr(i) = self.productElement(i).asInstanceOf[Object]
      i += 1
    arr

  def fromArray(xs: Array[Object]): Tuple = Tuple.fromArray(xs)
  def fromIArray(xs: IArray[Object]): Tuple = Tuple.fromIArray(xs)
  def fromProduct(xs: Product): Tuple = Tuple.fromProduct(xs)

  def cons(x: Any, self: Tuple): Tuple =
    val elems = elements(self)
    val arr = new Array[Object](elems.length + 1)
    arr(0) = x.asInstanceOf[Object]
    copyInto(elems, arr, 1)
    tuple(arr)

  def append(x: Any, self: Tuple): Tuple =
    val elems = elements(self)
    val arr = new Array[Object](elems.length + 1)
    copyInto(elems, arr, 0)
    arr(elems.length) = x.asInstanceOf[Object]
    tuple(arr)

  def concat[This <: Tuple, That <: Tuple](self: This, that: That): Tuple =
    val left = elements(self)
    val right = elements(that)
    if left.length == 0 then that
    else if right.length == 0 then self
    else
      val arr = new Array[Object](left.length + right.length)
      copyInto(left, arr, 0)
      copyInto(right, arr, left.length)
      tuple(arr)

  def size(self: Tuple): Int = self.asInstanceOf[Product].productArity

  def tail(self: Tuple): Tuple =
    val elems = nonEmpty(self)
    tuple(slice(elems, 1, elems.length))

  def init(self: Tuple): Tuple =
    val elems = nonEmpty(self)
    tuple(slice(elems, 0, elems.length - 1))

  def last(self: Tuple): Any =
    val p = self.asInstanceOf[Product]
    p.productElement(p.productArity - 1)

  def apply(self: Tuple, n: Int): Any = self.asInstanceOf[Product].productElement(n)

  def reverse(self: Tuple): Tuple =
    val elems = elements(self)
    // scalac's returns a tuple of one element itself.
    if elems.length <= 1 then return self
    val arr = new Array[Object](elems.length)
    var i = 0
    while i < arr.length do
      arr(i) = elems(elems.length - 1 - i)
      i += 1
    tuple(arr)

  def zip(t1: Tuple, t2: Tuple): Tuple =
    val left = elements(t1)
    val right = elements(t2)
    val arr = new Array[Object](if left.length < right.length then left.length else right.length)
    var i = 0
    while i < arr.length do
      arr(i) = (left(i), right(i))
      i += 1
    tuple(arr)

  def map[F[_]](self: Tuple, f: [t] => t => F[t]): Tuple =
    val elems = elements(self)
    val arr = new Array[Object](elems.length)
    var i = 0
    while i < arr.length do
      arr(i) = f(elems(i)).asInstanceOf[Object]
      i += 1
    tuple(arr)

  def take(self: Tuple, n: Int): Tuple =
    if n < 0 then throw new IndexOutOfBoundsException(n.toString)
    val elems = elements(self)
    tuple(slice(elems, 0, if n < elems.length then n else elems.length))

  def drop(self: Tuple, n: Int): Tuple =
    if n < 0 then throw new IndexOutOfBoundsException(n.toString)
    val elems = elements(self)
    tuple(slice(elems, if n < elems.length then n else elems.length, elems.length))

  def splitAt(self: Tuple, n: Int): (Tuple, Tuple) =
    if n < 0 then throw new IndexOutOfBoundsException(n.toString)
    val elems = elements(self)
    val at = if n < elems.length then n else elems.length
    (tuple(slice(elems, 0, at)), tuple(slice(elems, at, elems.length)))

  def consIterator(head: Any, tail: Tuple): Iterator[Any] = Iterator.single(head) ++ tail.asInstanceOf[Product].productIterator

  def concatIterator(tup1: Tuple, tup2: Tuple): Iterator[Any] = tup1.asInstanceOf[Product].productIterator ++ tup2.asInstanceOf[Product].productIterator

  // The lean tuple classes extend `NonEmptyTuple`, so one type test answers what scalac's
  // tests of the classes arity by arity answer.
  def isInstanceOfTuple(x: Any): Boolean = x.isInstanceOf[Tuple]
  def isInstanceOfEmptyTuple(x: Any): Boolean = x == EmptyTuple
  def isInstanceOfNonEmptyTuple(x: Any): Boolean = x.isInstanceOf[NonEmptyTuple]

  // A tuple's elements, a TupleXXL's own array, which no caller writes to.
  private def elements(self: Tuple): Array[Object] = (self: Any) match
    case self: TupleXXL => unsafeCast(self.elems)
    case _ => productToArray(self.asInstanceOf[Product])

  // scalac's `tail` and `init` of the empty tuple fail in their match over the tuple classes.
  private def nonEmpty(self: Tuple): Array[Object] =
    if self == EmptyTuple then throw new MatchError(self)
    elements(self)

  private def tuple(arr: Array[Object]): Tuple = Tuple.fromIArray(unsafeCast[Array[Object], IArray[Object]](arr))

  private def copyInto(from: Array[Object], to: Array[Object], at: Int): Unit =
    var i = 0
    while i < from.length do
      to(at + i) = from(i)
      i += 1

  // Elements `from` until `until` as a new array.
  private[runtime] def slice(xs: Array[Object], from: Int, until: Int): Array[Object] =
    val out = new Array[Object](until - from)
    var i = from
    while i < until do
      out(i - from) = xs(i)
      i += 1
    out
