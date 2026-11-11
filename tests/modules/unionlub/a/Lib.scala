package ula

// A union's signature name is its erased lub's (`sigName` over the `OrType` arm's `erasedLub`): a polymorphic function
// type with `Null` is its function class, which an inline caller's selection of the overload names; a union of two
// `Pure` classes is `Object`, a match type's array bound is kept and a value class with `Null` is `Object`, as the
// descriptors a client compiled against the declarations calls; a JDK class's union is lubbed by the class's own base
// classes (`List | ArrayList` is `List`), a class named only through an opaque type or a wildcard's bound too.
// An intersection with `Pure` erases as its other side (TypeErasure.scala:801-804): `(Pure | Null) & T` is `T` and
// `Pure & zz` is `zz`, whatever the names' order, and `(U1 | U2 | Null) & T` is `T` through the join of `U1 | U2`.
trait T
class B extends T
class PA extends scala.caps.Pure
class PB extends scala.caps.Pure
class S(val s: String) extends AnyVal
class SC extends java.io.Serializable
trait zz
class PT extends T with scala.caps.Pure
class PZ extends zz with scala.caps.Pure
trait U1 extends Any with scala.caps.Pure
trait U2 extends Any with scala.caps.Pure
class TU1 extends T with U1
type M[X] <: Array[T] = X match
  case String => Array[T]

object Ids:
  opaque type Id = java.util.UUID
  def of(u: java.util.UUID): Id = u

object L:
  type P = [A] => A => A
  def f(x: P | Null): Int = 1
  def f(x: String): Int = 2
  inline def call(x: P | Null): Int = f(x)
  def pure(x: PA | PB): String = "pure"
  def matched[X](x: M[X] | Array[B]): String = "matched"
  def nullable(x: S | Null): String = if x == null then "null" else "s"
  def size(x: java.util.List[Int] | java.util.ArrayList[Int]): Int = x.size
  def find(id: Ids.Id | Null): String = if id == null then "none" else "found"
  def late(x: Array[? <: java.util.concurrent.TimeoutException] | Array[Null]): Int = 1
  def serial(x: SC | Null): String = "serial"
  def pureNull(x: (scala.caps.Pure | Null) & T): String = "pure-null"
  def pureLate(x: scala.caps.Pure & zz): String = "pure-late"
  def pureUnionNull(x: (U1 | U2 | Null) & T): String = "pure-union-null"
