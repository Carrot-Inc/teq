package bea

trait Foo:
  def name: String = "foo"
trait Bar
class FB extends Foo with Bar:
  override def name: String = "fb"
class V(val n: Int) extends AnyVal
class W[A](val value: A) extends AnyVal

// Each erases as its bound does: a scalac-built caller names the descriptor scalac would write.
object Api:
  def p1[T <: Int](t: T): T = t
  def p3[T <: String](t: T): Int = t.length
  def p4[T <: Foo & Bar](t: T): String = t.name
  def p5[T <: Tuple](t: T): T = t
  def p6[T <: V](t: T): Int = t.n
  def p9[T <: Seq[Int]](t: T): Int = t.size
  def b2[T <: String, U <: T](t: T, u: U): U = u
  def b3[T <: Comparable[T]](a: T, b: T): Int = a.compareTo(b)
  def b5[F[X] <: Seq[X]](f: F[Int]): Int = f.length
  def b9[T <: Unit](t: T): T = t
  def b10[T <: W[String]](t: T): String = t.value
  def b11[T <: W[Int]](t: T): Int = t.value + 1
  def boxed[T <: V](x: T): Any = x
  def a1[T <: String](x: Array[T]): Int = x.length
  def a2[T <: Int](x: Array[T]): Int = x(0)
  def a5[T <: V](x: Array[T]): Int = x(0).n
  def w1(w: W[String]): W[String] = new W(w.value + "!")
  def w2(w: W[Int]): Int = w.value * 2

class Box[T <: Foo](t: T):
  def get: T = t
  def put(x: T): String = x.name
class FBBox extends Box[FB](new FB):
  override def get: FB = new FB
  override def put(x: FB): String = "fb box"

case class C3[T <: Int](t: T)
class C1[T <: String](val t: T, var u: T)

trait Holder:
  type T <: Foo
  def get: T
class FooHolder extends Holder:
  type T = FB
  def get: FB = new FB

// A cons tuple erases as it is spelled: `Product` where the chain ends in `EmptyTuple`'s alias
// or anything but a tuple, the tuple class where it ends in `EmptyTuple.type`.
object Cons:
  def q1(t: Int *: String *: EmptyTuple): Int = t._1 + t._2.length
  def q3(t: Int *: String *: EmptyTuple.type): Int = t._1
  def q5(t: Int *: EmptyTuple): Int = t._1
  def q7(t: Int *: Tuple): Int = 7
  def q9(t: Int *: (String, Long)): Long = t._3
  def q10(): Int *: String *: EmptyTuple = (1, "s")
  def q22(xs: Array[Int *: EmptyTuple]): Int = xs.length
  val v: Int *: EmptyTuple = Tuple1(7)
  inline def call: Int = q1((2, "inline"))

// `Nothing` and `Null` as written erase to `Nothing$` and `Null$`.
object Bottoms:
  def fail(msg: String): Nothing = throw new IllegalStateException(msg)
  def n: Null = null
  def orNull(c: Boolean): String = if c then "set" else n
  def bn[T <: Null](t: T): T = t
  def takeNull(x: Null): Int = 2

// A union erases to scalac's lub of its parts.
class U0:
  def name: String = "u0"
class UA extends U0:
  override def name: String = "ua"
class UB extends U0:
  override def name: String = "ub"
object Unions:
  def u1(x: UA | UB): String = x.name
  def u2(x: String | Null): Int = if x == null then -1 else x.length
  def u5(xs: Array[UA | UB]): Int = xs.length
  def u9[T <: UA](x: T | UB): String = x.name
  def pick(a: Boolean): UA | UB = if a then new UA else new UB

// A bridge over a parameter bounded by a value class unboxes and boxes it as the class.
trait VBase[A]:
  def f(x: A): A
class VImpl[A <: V] extends VBase[A]:
  def f(x: A): A = x
