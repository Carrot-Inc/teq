// A JS class local to a method, and one an inline method's every expansion defines again, keeps
// the members JavaScript reads by name.
import scala.scalajs.js
def make(n: Int): js.Object =
  class Local extends js.Object:
    def value: Int = n
    def twice: Int = n * 2
  new Local
transparent inline def made(n: Int): js.Object =
  class Inl extends js.Object:
    def value: Int = n + 1
  new Inl
@main def run(): Unit =
  val d = make(5).asInstanceOf[js.Dynamic]
  println(d.value)
  println(d.twice)
  println(made(1).asInstanceOf[js.Dynamic].value)
  println(made(7).asInstanceOf[js.Dynamic].value)
