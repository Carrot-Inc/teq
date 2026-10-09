package cfa

// The casts the code review's passes followed through products: arrays of `Nothing` cast to a trait,
// type-parameter bounds erased through `Tuple`, generic arrays, a spelled `*:` chain read through its
// tuple class, primitive casts with no conversion, `BoxedUnit` with a `null` argument, trait and
// `java.io.Serializable` casts, each in an ordinary method and an inline one the downstream expands.
trait Parent
trait Child extends Parent
class Good extends Child
case class Record(n: Int)
class Derived(n: Int) extends Record(n)
case object Marker
object Lib:
  var effects = 0
  def attempt(label: String)(body: => Any): Unit =
    try { body; println(label + " ok") }
    catch case _: ClassCastException => println(label + " CCE")
  def array(x: Array[Array[Nothing]]): Unit = { x.asInstanceOf[Parent]; () }
  inline def arrayInline(x: Array[Array[Nothing]]): Unit = { x.asInstanceOf[Parent]; () }
  def bound[A <: Tuple, B <: A](x: Any): Unit = { x.asInstanceOf[B]; () }
  inline def boundInline[A <: Tuple, B <: A](x: Any): Unit = { x.asInstanceOf[B]; () }
  def arrayBound[A <: AnyVal](x: Any): Unit = { x.asInstanceOf[Array[A]]; () }
  def arrayRefBound[A <: Tuple](x: Any): Unit = { x.asInstanceOf[Array[A]]; () }
  inline def primitive[A, B](inline x: A): B = x.asInstanceOf[B]
  def flag: Boolean = { effects += 1; false }
  def unit: Unit = { effects += 10; () }
  def boxed[A <: scala.runtime.BoxedUnit](x: Any): A = x.asInstanceOf[A]
  inline def boxedInline[A <: scala.runtime.BoxedUnit](x: Any): A = x.asInstanceOf[A]
  def nullBox: scala.runtime.BoxedUnit = { effects += 100; null.asInstanceOf[scala.runtime.BoxedUnit] }
  def inherited(x: Any): Unit = { x.asInstanceOf[Child]; () }
  inline def inheritedInline(x: Any): Unit = { x.asInstanceOf[Parent]; () }
  def serial(x: Any): Boolean = { x.asInstanceOf[java.io.Serializable]; x.isInstanceOf[java.io.Serializable] }
  inline def serialInline(x: Any): Boolean = { x.asInstanceOf[java.io.Serializable]; x.isInstanceOf[java.io.Serializable] }
  def accept[A <: java.io.Serializable](a: A): Boolean = a != null
  def head(x: Int *: Int *: EmptyTuple): Int = x._1
  inline def headInline(x: Int *: Int *: EmptyTuple): Int = x._1
