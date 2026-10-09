trait A[T]:
  def f: T
trait B[T: A]:
  println("B body: " + summon[A[T]].f)
  def g: String = "g"
trait Plain:
  println("Plain body")
  def h = 1
trait Deeper extends Plain:
  println("Deeper body")
  lazy val x = { println("Deeper.x"); 123 }
given a1: A[Int]:
  def f = 1
class D extends B[Int] with Plain:
  given a2: A[Int]:
    def f = 2
  println("D body")
class E extends Deeper with Plain:
  println("E body")
object O extends Deeper
enum Color extends Plain:
  case Red, Green
enum Shape(val n: Int) extends Deeper:
  case Circle extends Shape(1)
  case Square extends Shape(4)
trait Sam:
  println("Sam body")
  def run(x: Int): Int
class F extends B[Int]:
  println("F body")
@main def main(): Unit =
  println("before D")
  val d = D()
  println(d.g)
  println("before E")
  E().x
  println("before O")
  println(O.h)
  println("before Color")
  println(Color.Green.h)
  println("before Shape")
  println(Shape.Square.n)
  println("before anon")
  val p = new Plain {}
  println(p.h)
  val s: Sam = (x: Int) => x + 1
  println(s.run(1))
  println("before F")
  F()
