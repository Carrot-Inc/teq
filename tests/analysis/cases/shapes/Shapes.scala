package sh

// Types whose representation zinc hashes by identity: an intersection, a union and a refinement
// met twice under one name are one structure, as scalac's caches make them.
trait A
trait B

class Shared:
  def f(x: A & B): A & B = x
  def f(x: Int): A & B = ???
  def u(x: A | B): A | B = x
  def g: AnyRef { type T = Int } = ???
  def g(i: Int): AnyRef { type T = Int } = ???
  def h(k: => Int, xs: String*): Option[List[? <: A]] = None
  def w(m: Map[String, ? >: Int <: AnyVal]): Unit = ()

object Consts:
  final val Max = 10
  final val Name = "teq"
  final val Ratio = 2.5
  final val Flag = true
  final val Big = 12345678901L
  final val Letter = 'q'
  val plain: Int = 3

class Poly[+T, -U, V <: AnyRef](val t: T):
  def map[R](f: T => R): Poly[R, U, V] = ???
  def both[X >: T](x: X): (X, U => Unit) = ???
  type Out = List[V]
  type Bounded >: Null <: AnyRef

abstract class Self:
  self: A =>
  def me: A = this
