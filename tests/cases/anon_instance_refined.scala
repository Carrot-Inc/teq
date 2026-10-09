// An anonymous class instance is typed as its parents refined by each member that overrides one
// of theirs with a strictly narrower type, as scalac's `classBound` does: `new Base { val v: A =
// a }` is a `Base { val v: A }`. A member of the same type, or a new one, refines nothing.
trait Base:
  val v: Any
  def w: Any

trait Ops:
  def m(x: Int): Any

type WithV[A] = Base { val v: A }
type WithM[A] = Ops { def m(x: Int): A }

def mk[A](a: A): WithV[A] = new Base { val v: A = a; def w: Any = a }
def mkM[A](a: A): WithM[A] = new Ops { def m(x: Int): A = a }

object Main:
  def main(args: Array[String]): Unit =
    val x = new Base { val v: Int = 1; def w: String = "w"; def extra = 2 }
    val i: Int = x.v + 1
    val s: String = x.w
    val o = new Ops { def m(x: Int): List[Int] = List(x) }
    println((mk(3).v + 1, mkM("s").m(0).length, i, s, o.m(4).head))
