//> using platform jvm
// A union erases each side before the lub of their classes (`TypeErasure`'s `OrType` arm, `erasedLub`): `this.type`
// is its class, a path's class `P.I` is `I`, and an opaque type outside its scope is its underlying type, so each
// union below with a sibling of a common trait erases to that trait, the method's descriptor scalac gives it. An
// opaque array is an array whose elements are lubbed (`IArray[A]` is `Array[? <: A]`), and two value classes under
// opaque types erase to `Object`. A match type erases as its bound, an array kept; `Pure` is no lub; a value class
// does not derive from `Object`, so its union with `Null` is `Object`, inside an array too, and so does a universal
// trait. The JDK's classes are lubbed by their own base classes, the class file's. A
// class whose written parents are universal traits (`Serializable`) still derives from `Object` (`ensureFirstIsClass`),
// and so does a universal trait with an ancestor that is none (`Comparable`, `Ordered` through it).
trait T
class B extends T
class A extends T:
  def m(x: this.type | B): String = "m"
class Outer:
  class I extends T
  class J extends T
object P extends Outer
class V(val n: Int) extends AnyVal
class W(val n: Int) extends AnyVal
class S(val s: String) extends AnyVal
class PA extends scala.caps.Pure
trait U extends Any
class UC extends U
class SC extends java.io.Serializable
trait U6 extends Any with Comparable[U6]
class PB extends scala.caps.Pure
type M[X] <: Array[T] = X match
  case String => Array[T]
object Ops:
  opaque type O = A
  opaque type OV = V
  opaque type OW = W
object Erased:
  def op(x: Ops.O | B): String = "op"
  def inner(x: P.I | P.J): String = "inner"
  def arrays(x: IArray[A] | IArray[B]): String = "arrays"
  def values(x: Ops.OV | Ops.OW): String = "values"
  def matched[X](x: M[X] | Array[B]): String = "matched"
  def pure(x: PA | PB): String = "pure"
  def nullable(x: S | Null): String = "nullable"
  def nullables(x: Array[S] | Array[Null]): String = "nullables"
  def universal(x: U | Null): String = "universal"
  def throwable(x: Exception | Error): String = "throwable"
  def serial(x: SC | Null): String = "serial"
  def comparable(x: U6 | Null): String = "comparable"
  def ordered(x: scala.math.Ordered[Int] | Null): String = "ordered"
  def main(args: Array[String]): Unit =
    val t = classOf[T]
    println(classOf[A].getMethod("m", t).invoke(new A, new B))
    println(Erased.getClass.getMethod("op", t).invoke(Erased, new B))
    println(Erased.getClass.getMethod("inner", t).invoke(Erased, new P.I))
    println(Erased.getClass.getMethod("arrays", classOf[Array[T]]).invoke(Erased, Array[T](new B)))
    println(Erased.getClass.getMethod("values", classOf[Object]).invoke(Erased, new V(1)))
    println(Erased.getClass.getMethod("matched", classOf[Array[T]]).invoke(Erased, Array[T](new B)))
    println(Erased.getClass.getMethod("pure", classOf[Object]).invoke(Erased, new PA))
    println(Erased.getClass.getMethod("nullable", classOf[Object]).invoke(Erased, null))
    println(Erased.getClass.getMethod("nullables", classOf[Array[Object]]).invoke(Erased, null))
    println(Erased.getClass.getMethod("universal", classOf[Object]).invoke(Erased, new UC))
    println(Erased.getClass.getMethod("throwable", classOf[Throwable]).invoke(Erased, null))
    println(Erased.getClass.getMethod("serial", classOf[SC]).invoke(Erased, null))
    println(Erased.getClass.getMethod("comparable", classOf[U6]).invoke(Erased, null))
    println(Erased.getClass.getMethod("ordered", classOf[scala.math.Ordered[?]]).invoke(Erased, null))
