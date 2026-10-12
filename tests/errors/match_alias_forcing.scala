// A variable a match alias's application holds is fully defined where the tree's type is
// simplified (dotty's `TypeOps.simplify`: `isFullyDefined(tp, ForceDegree.all)` on an
// `isMatchAlias` application), so `lit(1)._1.x` is an `Int`; `compareS` gives `?N` of `S[?N]` the
// predecessor of a positive constant and nothing for `0`: each refused as scalac refuses it.
import scala.compiletime.ops.int.S
class Box[A](val x: A)
type Lit[T] = T match
  case 1 => String
  case _ => Int
type Other[T] = T match
  case String => Int
  case _ => Boolean
type Same[T] = T match
  case Int => Box[T]
  case _ => Box[T]
def lit[T <: Int](x: T): (Box[T], Lit[T]) = (new Box(x), null.asInstanceOf[Lit[T]])
def other[T](x: T): (Box[T], Other[T]) = (new Box(x), true.asInstanceOf[Other[T]])
def same[T](x: T): Same[T] = new Box(x).asInstanceOf[Same[T]]
def prev[N <: Int](n: S[N]): Box[N] = new Box((n - 1).asInstanceOf[N])
def pred[N <: Int](n: S[N]): N = (n - 1).asInstanceOf[N]
@main def run() =
  val a: 1 = lit(1)._1.x
  val b: 1 = other(1)._1.x
  val c: 1 = same(1).x
  val d = prev(2)
  val e: Box[1] = d
  println(pred(0))
// expect: 22:14: error: type mismatch: found Int, required 1
// expect: 23:14: error: type mismatch: found Int, required 1
// expect: 24:14: error: type mismatch: found Int, required 1
// expect: 26:19: error: type mismatch: found Box[Int], required Box[1]
// expect: 27:16: error: type mismatch: found 0, required S[N]
