// A program's class that is a function through its own trait is a function as it stands, called through
// the trait's `apply` and passed where a function is expected; a lambda takes the trait's single
// abstract method.
trait F extends (Int => Int):
  def apply(x: Int): Int
class G extends F:
  def apply(x: Int) = x + 1
trait H extends (Int => Int)
class K extends H:
  def apply(x: Int) = x * 10
@main def run(): Unit =
  val f: F = new G
  println(f(1))
  val h: H = new K
  println(h(2))
  val u: F = x => x * 2
  println(u(3))
  println(List(1, 2).map(f))
  println(List(1, 2).map(h))
  println((f.andThen(h))(4))
