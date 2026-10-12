// A value export forwards the type members of its path as aliases, `type T = x.T` (dotty's
// `addForwarder` of a type), named or by the wildcard, which leaves the private ones out.
class A:
  type T = Int
  type F[X] = List[X]
  class Box(val v: Int)
  private type P = String
class B(val x: A):
  export x._
  def show(): String =
    val t: T = 3
    val f: F[String] = List("a")
    val b: Box = new x.Box(5)
    s"$t $f ${b.v}"
class C(val x: A):
  export x.{T => U, F}
  def show(): String =
    val u: U = 6
    val f: F[Int] = List(7)
    s"$u $f"
object Main:
  def main(args: Array[String]): Unit =
    val b = B(A())
    val n: b.T = 4
    println(b.show())
    println(n)
    println(C(A()).show())
