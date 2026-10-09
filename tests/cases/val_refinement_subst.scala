// The `val` refinement of an applied alias takes its arguments on every substitution: a method's
// type parameters instantiated at a call, a class's seen from an instance, and a member read
// through the alias.
trait Base:
  val v: Any

type WithV[A] = Base { val v: A }

def mk[A](a: A): Base { val v: A } = new Base { val v: A = a }.asInstanceOf[Base { val v: A }]

def read(x: WithV[Int]): Int = x.v
def readAny[A](x: WithV[A]): A = x.v

class Box[A](a: A):
  def get: WithV[A] = mk(a)

object Main:
  def main(args: Array[String]): Unit =
    val r: WithV[Int] = new Box(5).get
    val i: Int = r.v
    val r2 = mk("s")
    val s: String = r2.v
    val l: List[Int] = readAny(mk(List(1, 2)))
    println(i)
    println(s)
    println(read(mk(7)))
    println(l)
