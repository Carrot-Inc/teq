// teq: --dialect no-scala2-implicits
// expect: the implicit definition `intShow` is not allowed under the dialect flag `no-scala2-implicits`: a Scala 2 implicit joins the candidates of every given search that a wildcard import of its scope reaches, and an implicit def with a plain parameter is a conversion; write it as a given
// expect: the implicit parameter clause of `show` is not allowed under the dialect flag `no-scala2-implicits`
trait Show[A]:
  def show(a: A): String

object Instances:
  implicit val intShow: Show[Int] = new Show[Int]:
    def show(a: Int): String = a.toString

def show[A](a: A)(implicit s: Show[A]): String = s.show(a)

@main def main(): Unit =
  import Instances._
  println(show(1))
