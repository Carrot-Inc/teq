// The descriptors of definitions whose erasure turns on a type parameter's bound, a value class's
// type arguments or an array's element: tests/classfile.scala compiles this file with teq and compares
// `javap -s -p` of every class with expected.txt, which is scalac 3.8.4's (see the suite's comment).
package erasure

trait Foo
trait Bar
class V(val n: Int) extends AnyVal
class W[A](val value: A) extends AnyVal
class WA[A](val value: Array[A]) extends AnyVal
class WS(val s: String) extends AnyVal

object Bounds:
  def p1[T <: Int](t: T): T = t
  def p2[T <: AnyVal](t: T): T = t
  def p3[T <: String](t: T): T = t
  def p4[T <: Foo & Bar](t: T): T = t
  def p5[T <: Tuple](t: T): T = t
  def p6[T <: V](t: T): T = t
  def p7[T <: AnyRef](t: T): T = t
  def p8[T](t: T): T = t
  def p9[T <: Seq[Int]](t: T): Int = t.size
  def b1[T, U <: T](t: T, u: U): U = u
  def b2[T <: String, U <: T](t: T, u: U): U = u
  def b3[T <: Comparable[T]](t: T): T = t
  def b4[F[_]](f: F[Int]): F[Int] = f
  def b5[F[X] <: Seq[X]](f: F[Int]): F[Int] = f
  def b8[T <: Singleton](t: T): T = t
  def b9[T <: Unit](t: T): T = t
  def b10[T <: W[String]](t: T): T = t
  def b11[T <: W[Int]](t: T): T = t
  def b12[T <: WA[String]](t: T): T = t
  def b13[T <: W[T]](t: T): T = t
  def b14[T <: WS](t: T): T = t
  def b15[T <: Long](t: T): T = t
  def b16[T <: Boolean](t: T): T = t
  def b17[T <: String | Int](t: T): T = t
  def b18[T <: Int & Singleton](t: T): T = t
  def b19[T <: 3](t: T): T = t
  def b20[T <: Foo](t: T, l: List[T]): List[T] = l
  def b21[T <: String](f: T => T): T => T = f
  def b22[T <: Function1[Int, Int]](f: T): T = f
  def b23[T >: String](t: T): T = t
  def b24[T <: NonEmptyTuple](t: T): T = t
  def b26[T <: (Int, String)](t: T): T = t
  def b27[T <: Array[Int]](t: T): T = t
  def b28[T <: Option[String]](t: T): T = t
  def b29[T <: Any](t: T): T = t
  def b30[T <: Matchable](t: T): T = t
  def b31[T <: Runnable](t: T): T = t
  def b32[T <: Foo, U <: T](u: U): T = u
  def boxed[T <: V](x: T): Any = x
  def unboxed[T <: W[String]](x: T): Any = x

object Arrays:
  def a1[T <: String](x: Array[T]): Array[T] = x
  def a2[T <: Int](x: Array[T]): Array[T] = x
  def a3[T <: AnyRef](x: Array[T]): Array[T] = x
  def a4[T](x: Array[T]): Array[T] = x
  def a5[T <: V](x: Array[T]): Array[T] = x
  def a6[T <: Foo & Bar](x: Array[T]): Array[T] = x
  def a7[T <: AnyVal](x: Array[T]): Array[T] = x
  def a8[T <: W[String]](x: Array[T]): Array[T] = x
  def a9[T <: Unit](x: Array[T]): Array[T] = x
  def a10[T <: Array[String]](x: Array[T]): Array[T] = x
  def a11[T <: Tuple](x: Array[T]): Array[T] = x
  def a12[T <: Null](x: Array[T]): Array[T] = x
  def a13[T <: Comparable[T]](x: Array[T]): Array[T] = x
  def a14[T <: Matchable](x: Array[T]): Array[T] = x
  def a15[T <: Singleton](x: Array[T]): Array[T] = x
  def a16[T <: String, U <: T](x: Array[U]): Array[U] = x
  def a17[T <: Long](x: Array[T]): Array[T] = x
  def a18[T <: Nothing](x: Array[T]): Array[T] = x
  def a19[T <: String | Int](x: Array[T]): Array[T] = x
  def a20[T <: Int | Long](x: Array[T]): Array[T] = x
  def a21[T <: Int & Singleton](x: Array[T]): Array[T] = x
  def a22[T <: Foo](x: Array[Array[T]]): Array[Array[T]] = x
  def a23[T](x: Array[Array[T]]): Array[Array[T]] = x
  def d1(x: Array[AnyVal]): Array[AnyVal] = x
  def d2(x: Array[Matchable]): Array[Matchable] = x
  def d3(x: Array[Foo & Bar]): Array[Foo & Bar] = x
  def d4(x: Array[String | Int]): Array[String | Int] = x
  def d5(x: Array[Any]): Array[Any] = x
  def d6(x: Array[V]): Array[V] = x

object Wrapped:
  def w1(x: W[String]): W[String] = x
  def w2(x: W[Int]): W[Int] = x
  def w3(x: W[W[String]]): Int = 1
  def w4(x: Array[W[String]]): Int = 1
  def w5(x: W[Unit]): Int = 1
  def w6(x: W[Array[Int]]): Int = 1
  def w7(x: W[V]): W[V] = x
  def w8(x: W[Any]): W[Any] = x
  def w9(x: WA[String]): WA[String] = x
  def w10(x: Singleton): Singleton = x

class C1[T <: String](val t: T, var u: T):
  def get: T = t
  def put(x: T): Unit = ()
case class C2[T <: Foo](t: T, n: Int)
case class C3[T <: Int](t: T)
case class C4[T <: V](t: T)
trait C5[T <: String]:
  val t: T
class C6 extends C5[String]:
  val t: String = "c6"

class Box[T <: Foo]:
  def get: T = ???
  def put(t: T): Unit = ()
class Sub extends Box[Foo & Bar]:
  override def get: Foo & Bar = ???
abstract class Box2[T <: Foo]:
  def get: T
class Sub2 extends Box2[Foo]:
  def get: Foo = ???
abstract class Box3[T]:
  def get: T
  def put(t: T): Unit
class Sub3 extends Box3[String]:
  def get: String = ""
  def put(t: String): Unit = ()
abstract class Box4[T <: AnyVal]:
  def get: T
class Sub4 extends Box4[Int]:
  def get: Int = 1
abstract class Box5[T <: Int]:
  def get(t: T): T
class Sub5 extends Box5[3]:
  def get(t: 3): 3 = t
trait Gen[T <: String]:
  def g(t: T): T
class GenS extends Gen[String]:
  def g(t: String): String = t
trait GenM:
  def m[T <: Foo](t: T): T
class GenMI extends GenM:
  def m[T <: Foo](t: T): T = t
object Sub6 extends Box2[Foo & Bar]:
  def get: Foo & Bar = ???

trait Holder:
  type T <: Foo
  def get: T
  def put(t: T): Unit
  type U <: String
  def u(x: U): U
  def ua(x: Array[U]): Array[U]
  type I <: Int
  def i(x: I): I
  def ia(x: Array[I]): Array[I]
  type A
  def aa(x: Array[A]): Array[A]

// A cons tuple erases by scalac's `tupleArity` of it as written: the tuple class where the chain
// ends in `EmptyTuple.type` or a tuple, `Product` where it ends in anything else, `EmptyTuple`'s
// alias among them.
object Cons:
  type E = EmptyTuple.type
  type P2 = (String, Long)
  def q1(t: Int *: String *: EmptyTuple): Int = 1
  def q2[H, T <: Tuple](t: H *: T): Int = 2
  def q3(t: Int *: String *: EmptyTuple.type): Int = 3
  def q4(t: (Int, String)): Int = 4
  def q5(t: Int *: EmptyTuple): Int = 5
  def q6(t: EmptyTuple): Int = 6
  def q7(t: Int *: Tuple): Int = 7
  def q8(t: Int *: String *: Tuple): Int = 8
  def q9(t: Int *: (String, Long)): Int = 9
  def q10(): Int *: String *: EmptyTuple = (1, "s")
  def q11(t: Int *: E): Int = 11
  def q12(t: Int *: P2): Int = 12
  def q13(t: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: EmptyTuple): Int = 13
  def q14(t: Array[Int *: EmptyTuple]): Array[Int *: EmptyTuple] = t
  def q15(t: Option[Int *: EmptyTuple]): Int = 15
  def q16(t: (Int, String) *: EmptyTuple.type): Int = 16
  def q17(t: Tuple): NonEmptyTuple = ???
  val v: Int *: EmptyTuple = Tuple1(1)
  var w: Int *: String *: EmptyTuple = (1, "w")

// `Nothing` and `Null` as a signature writes them erase to scala-library's `Nothing$` and `Null$`.
object Bottoms:
  def fail(msg: String): Nothing = throw new RuntimeException(msg)
  def n: Null = null
  def takeNothing(x: Nothing): Int = 1
  def takeNull(x: Null): Int = 2
  def bn[T <: Null](t: T): T = t
  def bt[T <: Nothing](t: T): T = t
  def arrays(x: Array[Null]): Array[Nothing] = ???

// A union erases to scalac's `erasedLub` of its parts: their common class, the other part beside
// `Nothing` and beside `Null` where that is a reference, two arrays of references an array of
// their elements' lub, anything else `Object`.
class U0
class UA extends U0
class UB extends U0
trait UT
trait UT2 extends UT
trait UT3 extends UT
class UC extends UT2
class UD extends UT3
object Unions:
  def u1(x: UA | UB): UA | UB = x
  def u2(x: String | Null): String | Null = x
  def u3(x: Int | Null): Int | Null = x
  def u4(x: Array[Int] | Array[String]): Int = 4
  def u5(x: Array[UA] | Array[UB]): Int = 5
  def u6(x: Array[Nothing] | Array[String]): Int = 6
  def u7(x: Array[UA | UB]): Int = 7
  def u8[T](x: T | String): Int = 8
  def u9[T <: UA](x: T | UB): Int = 9
  def u10(x: UC | UD): Int = 10
  def u11(x: UA | Nothing): Int = 11
  def u12(x: Int | Long): Int = 12
  def u13(x: UA | UB | U0): Int = 13
  def u14(x: List[Int] | Vector[Int]): Int = 14
  def u15(x: Some[Int] | None.type): Int = 15
  def u16(x: Option[Int] | Null): Int = 16
  def u17[T <: String](x: T | Null): Int = 17
  def u18(x: Array[UA] | Null): Int = 18
  def u19(x: Array[Array[Null] | Array[String]]): Int = 19
  def u20[T <: UA | UB](x: T): T = x
  def u21(x: UT2 | UT3): Int = 21
  def u22(x: UA | UT2): Int = 22
  def u23(x: Array[Int] | Array[Int]): Int = 23

// A value class over an applied higher-kinded parameter holds the class its argument names; a
// cons chain's tail that is a union or an intersection of tuples of one arity has that arity.
class HK[F[_], A](val value: F[A]) extends AnyVal
object Applied:
  def g1(x: HK[List, Int]): HK[List, Int] = x
  def g2(x: HK[Option, String]): Int = 2
  def t1(x: Int *: ((String, Int) | (Long, Int))): Int = 1
  def t2(x: Int *: ((String, Int) | Tuple1[Long])): Int = 2

// Unions nested in unions, intersections and arrays, and value classes over constructors.
trait UMark
object Nested:
  def n1(x: Array[UA] | Array[UB] | Array[U0]): Array[UA] | Array[UB] | Array[U0] = x
  def n2(x: HK[Array, Int]): HK[Array, Int] = x
  def n3(x: HK[[X] =>> X, String]): HK[[X] =>> X, String] = x
  def n4(x: (UA & UMark) | UB): (UA & UMark) | UB = x
  def n5[T <: UA, U <: T, V <: T](x: (U | V) | UB): (U | V) | UB = x
  def n6(x: HK[[X] =>> Either[String, X], Int]): Int = 6
  def n7(x: Array[UA | UB] | Array[U0]): Int = 7
  def n8(x: Array[Array[UA]] | Array[Array[UB]]): Int = 8
  def n9(x: Unit | Null): Int = 9
  def n10(x: (UA | Null) | (UB | Null)): Int = 10

// More shapes: a union of value classes as a bound, a higher-kinded parameter
// bounded by an array, a dependent result spelled as a cons chain, an intersection of value
// classes, unions behind arrays and bounds of jar classes, and the lub's walk ending at the first
// trait.
class V2(val n: Int) extends AnyVal
trait LTr
class LP
class LA extends LP with LTr
class LB extends LP with LTr
object Third:
  def f1[T <: V | V2](x: T): T = x
  def f2[F[X] <: Array[X]](x: F[Int]): F[Int] = x
  def f3[F[X] <: Array[X]](x: F[String]): Int = 3
  def dependent(x: Int *: EmptyTuple): x.type = x
  def g1[T <: V & V2](x: T): Int = 1
  def g2(x: Array[List[Int]] | Array[Vector[Int]]): Int = 2
  def g3[T <: List[Int] | Vector[Int]](x: Array[T]): Int = 3
  def g4(x: scala.collection.immutable.TreeSeqMap[Int, Int] | scala.collection.immutable.VectorMap[Int, Int]): Int = 4
  def g5(x: LA | LB): LA | LB = x
