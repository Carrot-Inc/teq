package fix.shapes

import scala.annotation.targetName

trait Show[-A]:
  def show(a: A): String
  extension (a: A) def shown: String = show(a)

object Show:
  given Show[Int] with
    def show(a: Int): String = a.toString
  given listShow: [A: Show] => Show[List[A]] = xs => xs.map(summon[Show[A]].show).mkString(",")
  given optShow[A](using s: Show[A]): Show[Option[A]] with
    def show(o: Option[A]): String = o.fold("-")(s.show)
  implicit val stringShow: Show[String] = s => s
  implicit def pairShow[A, B](implicit a: Show[A], b: Show[B]): Show[(A, B)] = p => a.show(p._1) + b.show(p._2)
  def apply[A](using s: Show[A]): Show[A] = s

sealed abstract class Shape(val name: String) extends Product with Serializable
final case class Circle(r: Double) extends Shape("circle")
case class Rect(w: Double, h: Double = 1.0) extends Shape("rect")
case object Empty extends Shape("empty")

enum Color(val rgb: Int):
  case Red extends Color(0xff0000)
  case Green extends Color(0x00ff00)
  case Custom(value: Int) extends Color(value)

enum Tree[+A]:
  case Leaf
  case Node(left: Tree[A], value: A, right: Tree[A])

class Box[+A <: AnyRef, F[_], -C >: Null](private val a: A, protected var count: Int)(using val ord: Ordering[Int]):
  self: Serializable =>
  type Elem >: Null <: AnyRef
  type Pair[X] = (X, X)
  lazy val size: Int = 1
  def byName(x: => Int, ys: String*): Int = x
  def curried[B >: A](f: A => B)(g: (B, Int) => String): String = ""
  inline def twice(inline n: Int): Int = n + n
  transparent inline def pick(b: Boolean): Any = if b then 1 else ""
  def dependent(b: Functor[F]): b.type = b
  def union(x: Int | String): A & Serializable = ???
  def ctx(f: Int ?=> String): String = f(using 1)
  def hk[G[_], T](g: G[T]): F[T] = ???
  def lambda(f: Functor[[X] =>> Either[String, X]]): Unit = ()
  def wildcard(xs: List[?], ys: List[? <: Shape]): Unit = ()
  def tuple: (Int, String, Double) = ???
  def refined: Box[A, F, C] { type Elem = String } = ???
  def poly: [T] => T => List[T] = ???
  @deprecated("old", "1.0") def old: Int = 0
  @targetName("plus") def +(other: Int): Int = other
  final override def toString: String = ""
  private[shapes] def scoped: Int = 0

object Opaques:
  opaque type Meters = Double
  opaque type Name <: String = String
  opaque type Wrapped[A] = List[A]
  object Meters:
    def apply(d: Double): Meters = d
    extension (m: Meters) def value: Double = m
  extension [A](xs: List[A])
    def second: A = xs.tail.head
    def +:+(x: A): List[A] = xs :+ x

object Exports:
  private val impl = new Impl
  class Impl:
    def run(x: Int): Int = x
    val const = 1
  export impl.{run, const as c}
  export Opaques.*

type Alias[A] = Map[String, A]
type Elem[X] = X match
  case String => Char
  case Array[t] => t
def topLevel(x: Int): Int = x
val topVal: Int = 1
given topGiven: Ordering[Shape] = Ordering.by(_.name)
extension (s: String) def shout: String = s.toUpperCase
trait Functor[F[_]]:
  extension [A](fa: F[A]) def fmap[B](f: A => B): F[B]
