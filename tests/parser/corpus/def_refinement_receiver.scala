// A `def` refinement takes the arguments of an applied alias as a `val` refinement does, and a
// member read on a refined receiver has the refinement's type, whether the receiver is a path
// (`r.v + 1`) or not (`mk(3).v`): the class's member is called, the refinement types it.
trait Base:
  val v: Any

trait Ops:
  def m(x: Int): Any
  def n: Any

type WithV[A] = Base { val v: A }
type WithM[A] = Ops { def m(x: Int): A; def n: List[A] }

def mk[A](a: A): WithV[A] = (new Base { val v: Any = a }).asInstanceOf[WithV[A]]
def mkM[A](a: A): WithM[A] = (new Ops { def m(x: Int): Any = a; def n: Any = List(a) }).asInstanceOf[WithM[A]]
def readM[A](x: WithM[A]): A = x.m(0)

class Box[A](a: A):
  def get: WithM[A] = mkM(a)

object Main:
  def main(args: Array[String]): Unit =
    val j: Int = mk(3).v
    val r: WithV[Int] = mk(4)
    val k: Int = r.v + 1
    val s: String = mkM("s").m(1)
    val t: List[String] = mkM("t").n
    val b: WithM[Int] = new Box(5).get
    val u: Int = b.m(2) + 1
    val l: List[Int] = readM(mkM(List(1, 2)))
    println((j, k, s, t, u, l))
    println(mkM("x").n.map(_.length))
