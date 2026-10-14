// A wildcard import selects the members accessible where it is read (dotty's import `selection`,
// `accessibleFrom`): an object's private alias or val is no binding of the import outside the
// object and its companion, so the other import's `A` and `size` are the only ones.
object Used {
  case class A(value: Int)
  val size = 3
}

object Unused {
  private type A = Used.A
  private val size = 7
  class B
  def own(a: A): Int = a.value + size
}

object Test {
  import Used.*
  import Unused.*

  def foo(a: A): Int = a.value
  def g(b: B): String = "B"
  def run(): Unit =
    println(foo(A(1)))
    println(size)
    println(g(new B))
    println(Unused.own(Used.A(2)))
}

@main def main(): Unit = Test.run()
