package scala

@js("$println($printed($0))")
@jvm("getstatic java/lang/System.out:Ljava/io/PrintStream; $0:L invokevirtual java/io/PrintStream.println(Ljava/lang/Object;)V")
def printlnImpl(x: Any): Unit

@predef
@js("$print($printed($0))")
@jvm("getstatic java/lang/System.out:Ljava/io/PrintStream; $0:L invokevirtual java/io/PrintStream.print(Ljava/lang/Object;)V")
def print(x: Any): Unit

// scala-library's two overloads: `println(())` passes the unit, which prints as Scala.js prints it,
// `undefined` (`$printed`), and no default takes it for a missing argument.
@predef
def println(): Unit = printlnImpl("")
@predef
def println(x: Any): Unit = printlnImpl(x)

@predef
def locally[T](x: T): T = x

// The supertypes of the tuple classes, which `inline match` walks as `h *: t` and `EmptyTuple`.
sealed trait Tuple:
  inline def toList: List[Tuple.Union[this.type]] =
    this.asInstanceOf[Product].productIterator.toList.asInstanceOf[List[Tuple.Union[this.type]]]
  // The members on a tuple whose length its type does not give (`Tuple.fromArray(xs)`, an
  // abstract `T <: Tuple`), which the runtime answers as scalac's does; on a tuple type the
  // compiler's own come first (`typer/arity.rs`). Typed by their bounds: scalac's match types of
  // an unknown tuple do not reduce.
  inline def toArray: Array[Object] = scala.runtime.Tuples.toArray(this)
  inline def toIArray: IArray[Object] = scala.runtime.Tuples.toIArray(this)
  inline def size: Int = scala.runtime.Tuples.size(this)
  inline def apply(n: Int): Any = scala.runtime.Tuples.apply(this, n)
  inline def head: Any = scala.runtime.Tuples.apply(this, 0)
  inline def last: Any = scala.runtime.Tuples.last(this)
  inline def tail: Tuple = scala.runtime.Tuples.tail(this)
  inline def init: Tuple = scala.runtime.Tuples.init(this)
  inline def *: [H](x: H): H *: Tuple = scala.runtime.Tuples.cons(x, this).asInstanceOf[H *: Tuple]
  inline def :* [L](x: L): Tuple = scala.runtime.Tuples.append(x, this)
  inline def ++ (that: Tuple): Tuple = scala.runtime.Tuples.concat(this, that)
  inline def zip(t2: Tuple): Tuple = scala.runtime.Tuples.zip(this, t2)
  inline def map[F[_]](f: [t] => t => F[t]): Tuple.Map[this.type, F] = scala.runtime.Tuples.map(this, f).asInstanceOf[Tuple.Map[this.type, F]]
  inline def take(n: Int): Tuple = scala.runtime.Tuples.take(this, n)
  inline def drop(n: Int): Tuple = scala.runtime.Tuples.drop(this, n)
  inline def splitAt(n: Int): (Tuple, Tuple) = scala.runtime.Tuples.splitAt(this, n)
  inline def reverse: Tuple = scala.runtime.Tuples.reverse(this)
sealed trait NonEmptyTuple extends Tuple
case object EmptyTuple extends Tuple:
  override def toString: String = "()"
type EmptyTuple = EmptyTuple.type

// `h *: t` is the tuple one longer than `t` where `t` is known, and an application of this trait where it is not (a
// match type case); the compiler relates the two and gives every tuple `head`, `tail`, `apply`, `size`, `++` and `zip`.
// One of open length (`tt: (h *: rest)`) reads `head` and `tail` as a product, on one line, which moves no line number.
sealed trait *:[+H, +T <: Tuple] extends NonEmptyTuple { override inline def head: H = this.asInstanceOf[Product].productElement(0).asInstanceOf[H]; override inline def tail: T = Tuple.fromArray(Array.tabulate[Any](this.asInstanceOf[Product].productArity - 1)(i => this.asInstanceOf[Product].productElement(i + 1))).asInstanceOf[T] }

object Tuple:
  import scala.compiletime.ops.int.S

  def apply(): EmptyTuple = EmptyTuple
  def apply[T](x: T): T *: EmptyTuple = Tuple1(x)
  def unapply(x: EmptyTuple): true = true

  def fromArray[T](xs: Array[T]): Tuple = xs.length match
    case 0 => EmptyTuple
    case 1 => Tuple1(xs(0))
    case 2 => (xs(0), xs(1))
    case 3 => (xs(0), xs(1), xs(2))
    case 4 => (xs(0), xs(1), xs(2), xs(3))
    case 5 => (xs(0), xs(1), xs(2), xs(3), xs(4))
    case 6 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5))
    case 7 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6))
    case 8 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7))
    case 9 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8))
    case 10 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9))
    case 11 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10))
    case 12 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11))
    case 13 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12))
    case 14 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13))
    case 15 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13), xs(14))
    case 16 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13), xs(14), xs(15))
    case 17 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13), xs(14), xs(15), xs(16))
    case 18 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13), xs(14), xs(15), xs(16), xs(17))
    case 19 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13), xs(14), xs(15), xs(16), xs(17), xs(18))
    case 20 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13), xs(14), xs(15), xs(16), xs(17), xs(18), xs(19))
    case 21 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13), xs(14), xs(15), xs(16), xs(17), xs(18), xs(19), xs(20))
    case 22 => (xs(0), xs(1), xs(2), xs(3), xs(4), xs(5), xs(6), xs(7), xs(8), xs(9), xs(10), xs(11), xs(12), xs(13), xs(14), xs(15), xs(16), xs(17), xs(18), xs(19), xs(20), xs(21))
    case _ => scala.runtime.TupleXXL.copyOf(xs)
  // An immutable array of more than 22 elements is the TupleXXL's own, as scalac shares it.
  def fromIArray[T](xs: IArray[T]): Tuple =
    val arr = unsafeCast[IArray[T], Array[T]](xs)
    if arr.length > 22 then scala.runtime.TupleXXL.fromIArray(unsafeCast(xs)) else fromArray(arr)
  // A tuple is its own, as scalac's runtime returns it.
  def fromProduct(product: Product): Tuple = (product: Any) match
    case t: NonEmptyTuple => t
    case _ => fromArray(Array.tabulate(product.productArity)(product.productElement))
  def fromProductTyped[P <: Product](p: P)(using m: scala.deriving.Mirror.ProductOf[P]): m.MirroredElemTypes =
    fromProduct(p).asInstanceOf[m.MirroredElemTypes]
  given canEqualEmptyTuple: CanEqual[EmptyTuple, EmptyTuple] = CanEqual.derived
  given canEqualTuple[H1, T1 <: Tuple, H2, T2 <: Tuple](
    using eqHead: CanEqual[H1, H2], eqTail: CanEqual[T1, T2]
  ): CanEqual[H1 *: T1, H2 *: T2] = CanEqual.derived

  type Head[X <: Tuple] = X match
    case x *: _ => x

  type Tail[X <: Tuple] <: Tuple = X match
    case _ *: xs => xs

  type Elem[X <: Tuple, N <: Int] = X match
    case x *: xs =>
      N match
        case 0 => x
        case S[n1] => Elem[xs, n1]

  type Size[X <: Tuple] <: Int = X match
    case EmptyTuple => 0
    case x *: xs => S[Size[xs]]

  type Map[Tup <: Tuple, F[_]] <: Tuple = Tup match
    case EmptyTuple => EmptyTuple
    case h *: t => F[h] *: Map[t, F]

  type InverseMap[X <: Tuple, F[_]] <: Tuple = X match
    case F[x] *: t => x *: InverseMap[t, F]
    case EmptyTuple => EmptyTuple

  type Concat[X <: Tuple, +Y <: Tuple] <: Tuple = X match
    case EmptyTuple => Y
    case x1 *: xs1 => x1 *: Concat[xs1, Y]

  type Zip[T1 <: Tuple, T2 <: Tuple] <: Tuple = (T1, T2) match
    case (h1 *: t1, h2 *: t2) => (h1, h2) *: Zip[t1, t2]
    case (EmptyTuple, _) => EmptyTuple
    case (_, EmptyTuple) => EmptyTuple
    case _ => Tuple

  type Fold[Tup <: Tuple, Z, F[_, _]] = Tup match
    case EmptyTuple => Z
    case h *: t => F[h, Fold[t, Z, F]]

  type Union[T <: Tuple] = Fold[T, Nothing, [x, y] =>> x | y]

  type IsMappedBy[F[_]] = [X] =>> X =:= Map[InverseMap[X & Tuple, F], F]

// The failures of the standard library go through these. In JavaScript the runtime throws the
// exception class when the program has it (a program that catches has them all) and a plain
// error otherwise, so a program that catches nothing carries no exception classes.
@js("$fail(\"RuntimeException\", $0)")
def runtimeError(message: String): Nothing = throw new RuntimeException(message)

@js("$fail(\"NoSuchElementException\", $0)")
def noSuchElement(message: String): Nothing = throw new NoSuchElementException(message)

@js("$fail(\"IndexOutOfBoundsException\", $0)")
def indexOutOfBounds(message: String): Nothing = throw new IndexOutOfBoundsException(message)

@js("$fail(\"UnsupportedOperationException\", $0)")
def unsupportedOperation(message: String): Nothing = throw new UnsupportedOperationException(message)

@js("$fail(\"IllegalArgumentException\", $0)")
def illegalArgument(message: String): Nothing = throw new IllegalArgumentException(message)

@js("$fail(\"AssertionError\", $0)")
def assertionFailed(message: String): Nothing = throw new AssertionError(message)

@js("$fail(\"NotImplementedError\", $0)")
def notImplemented(message: String): Nothing = throw new NotImplementedError(message)

@predef
def ??? : Nothing = notImplemented("an implementation is missing")

@predef
def assert(condition: Boolean): Unit =
  if !condition then assertionFailed("assertion failed")

@predef
def assert(condition: Boolean, message: => Any): Unit =
  if !condition then assertionFailed("assertion failed: " + message.toString)

@predef
def require(condition: Boolean): Unit =
  if !condition then illegalArgument("requirement failed")

@predef
def require(condition: Boolean, message: => Any): Unit =
  if !condition then illegalArgument("requirement failed: " + message.toString)

@predef
def summon[T](using x: T): T = x

@predef
inline def valueOf[T](using vt: ValueOf[T]): T = vt.value

@predef
def implicitly[T](implicit x: T): T = x

/** An implicit conversion: a given of this type converts a `T` where a `U` is needed. */
trait Conversion[-T, +U] extends (T => U)

@predef
def identity[A](x: A): A = x

type Serializable = java.io.Serializable

// What a case class, a case object, an enum case or a tuple is: its elements by position. Those
// classes are products without extending the trait, which the compiler knows; a class of the
// program that extends it implements the members itself. A call of a concrete member that
// resolves to the trait's own goes to the function below it, since a product by rule has no
// body of the trait.
trait Equals:
  def canEqual(that: Any): Boolean

trait Product extends Equals:
  @js("$productArity($0)")
  @jvm("$0:L invokeinterface scala/Product.productArity()I")
  def productArity: Int
  @js("$productElement($0, $1)")
  @jvm("$0:L $1:I invokeinterface scala/Product.productElement(I)Ljava/lang/Object;")
  def productElement(n: Int): Any
  @js("$productPrefix($0)")
  @jvm("$0:L invokeinterface scala/Product.productPrefix()Ljava/lang/String;")
  def productPrefix: String
  // A case class's field names, which its registration carries once a program asks for one; a
  // class that defines the method answers itself, any other product with empty names. The JVM
  // takes them from scala-library's `Product` alone.
  @js("$productElementName($0, $1)")
  def productElementName(n: Int): String
  def productElementNames: Iterator[String] = productElementNamesImpl(this)
  def productIterator: Iterator[Any] = productIteratorImpl(this)
  def canEqual(that: Any): Boolean = canEqualImpl(this, that)

def productIteratorImpl(p: Product): Iterator[Any] = Iterator.tabulate(p.productArity)(p.productElement)
def productElementNamesImpl(p: Product): Iterator[String] = Iterator.tabulate(p.productArity)(p.productElementName)
// What scalac's case class answers: `that.isInstanceOf[C]`.
def canEqualImpl(p: Equals, that: Any): Boolean = that != null && p.getClass == that.getClass

extension [A](a: A)
  @predef
  def ->[B](b: B): (A, B) = (a, b)

extension [T](x: T | Null)
  inline def nn: T = x.asInstanceOf[T]
extension (x: AnyRef | Null)
  inline infix def eq(y: AnyRef | Null): Boolean = x.asInstanceOf[AnyRef] eq y.asInstanceOf[AnyRef]
  inline infix def ne(y: AnyRef | Null): Boolean = !(x.asInstanceOf[AnyRef] eq y.asInstanceOf[AnyRef])

@predef
object Function:
  def const[T, U](x: T)(y: U): T = x
  def chain[T](fs: Seq[T => T]): T => T = x => fs.foldLeft(x)((acc, f) => f(acc))
  def tupled[A, B, R](f: (A, B) => R): ((A, B)) => R = p => f(p._1, p._2)
  def untupled[A, B, R](f: ((A, B)) => R): (A, B) => R = (a, b) => f((a, b))
  def uncurried[A, B, R](f: A => B => R): (A, B) => R = (a, b) => f(a)(b)
  def unlift[A, B](f: A => Option[B]): PartialFunction[A, B] = unliftFunction(f)



// `case k -> v` takes a pair apart, as scala-library's `object ->` has it.
@predef
object -> :
  def unapply[A, B](x: (A, B)): Some[(A, B)] = Some(x)

// The extractor of `h *: t` patterns, as scala-library's; after everything else, moving no line number.
object *: { def unapply[H, T <: Tuple](x: H *: T): (H, T) = (x.head, x.tail) }
