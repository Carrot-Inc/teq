// A refinement's members name each other and the parent's members wherever they stand: dotty
// types them in the refinement class's scope (`Typer.typedRefinedTypeTree`) and closes the
// refined type over its `this` (`TypeAssigner.assignType`, `RecType.closeOver`).
type R = AnyRef { type T; type U = T }
trait HasU { def u(): Any }
type Y = HasU { type T = String; def u(): T }
type X[T] = ({ type F[_]; type R = F[T] })#R
type O1[A] = { type OutInner[B] = List[B]; type Out = OutInner[A] }

trait Base { type T; def get: T }
type Inherits = Base { type U = T }

object noRecursionLimit:
  type M = { type T[+A]; type Ev >: T[Any] <: T[Nothing] }
  val M: M = ().asInstanceOf[M]
  def dcast(m: M.T[Any]): M.T[Int] = m: M.Ev

class Box(v: String) extends Base:
  type T = String
  type U = String
  def get: T = v

def same(r: R)(x: r.T): r.U = x
def through(b: Inherits)(x: b.U): b.T = x

@main def run(): Unit =
  val b = Box("ok")
  val r: R { type T = String } = b
  val s: String = same(r)("same")
  println(s)
  val y: Y = new HasU { type T = String; def u(): T = "u" }
  val u: String = y.u()
  println(u)
  val o: O1[Int]#Out = List(1, 2)
  println(o)
  val i: Inherits { type T = String } = b
  println(through(i)(i.get))
  println(noRecursionLimit.M != null)
