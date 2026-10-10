// Classes nested in classes reached through a subclass or a prefix: a parent constructor of an
// inherited inner class seen from the subclass, a mirror's product made on its prefix, an object
// inheriting an inner trait's members through an import, `p.type#T` as `p.T`, an object
// overriding a val that a nested type is seen through, and a lambda whose result names its
// parameter's inner class against a plain function type.
import scala.deriving.Mirror

trait Num[T]:
  def plus(a: T, b: T): T
  class Ops(val lhs: T):
    def +(r: T): T = plus(lhs, r)
trait Integ[T] extends Num[T]:
  class IOps(lhs: T) extends Ops(lhs):
    def twice: T = plus(lhs, lhs)
object IntI extends Integ[Int]:
  def plus(a: Int, b: Int) = a + b

class Shapes(val tag: String):
  sealed trait Shape
  case class Box(w: Int) extends Shape:
    def owner: String = tag
  case object Dot extends Shape

trait Outer:
  trait Inner:
    val v: Int = 3
    object Deep:
      val w: Int = 4
object Holder extends Outer:
  object In extends Inner

abstract class Cell:
  type T = Node
  class Node:
    val foo = 1

trait D:
  trait Manifest:
    class Entry(val n: Int)
  val M: Manifest
  def m: M.Entry = new M.Entry(5)
object D1 extends D:
  object M extends Manifest

trait Foo:
  def name: String
class Bar(val n: String):
  class Baz extends Foo:
    def name = n
  def baz: Baz = new Baz

@main def run(): Unit =
  val i = new IntI.IOps(3)
  println(i + 4)
  println(i.twice)

  val s = new Shapes("s1")
  val m = summon[Mirror.ProductOf[s.Box]]
  val b: s.Box = m.fromProduct(Tuple1(9))
  println(b.w)
  println(b.owner)
  val sum = summon[Mirror.SumOf[s.Shape]]
  println(sum.ordinal(s.Dot))

  import Holder.In.*
  println(v)
  println(Deep.w)

  val cell = new Cell {}
  val n: cell.Node = new cell.type#T()
  println(n.foo)

  val e: D1.M.Entry = D1.m
  println(e.n)

  def frob[P1, P2 <: Foo](f: P1 => P2, p: P1): String = f(p).name
  println(frob((p: Bar) => p.baz, new Bar("bar")))
