import scala.language.implicitConversions
trait Show[A] { def show(a: A): String }
object Show {
  implicit val showInt: Show[Int] = new Show[Int] { def show(a: Int) = s"Int($a)" }
  implicit def showList[A](implicit s: Show[A]): Show[List[A]] = new Show[List[A]] {
    def show(as: List[A]) = as.map(s.show).mkString("[", ", ", "]")
  }
  implicit object showBool extends Show[Boolean] { def show(b: Boolean) = if (b) "yes" else "no" }
  implicit lazy val showStr: Show[String] = new Show[String] { def show(s: String) = s"'$s'" }
}
trait Ord[A] { def lt(a: A, b: A): Boolean }
object Ord {
  implicit object IntOrd extends Ord[Int] { def lt(a: Int, b: Int) = a < b }
}
object Lib {
  def show[A](a: A)(implicit s: Show[A]): String = s.show(a)
  def max[A](a: A, b: A)(implicit ord: Ord[A]): A = if (ord.lt(a, b)) b else a
  def describe[A: Show](a: A): String = "desc " + implicitly[Show[A]].show(a)
  implicit class RichInt(private val n: Int) {
    def squared: Int = n * n
    def times(k: Int): Int = n * k
  }
  implicit class ShowOps[A](a: A)(implicit s: Show[A]) {
    def shown: String = s.show(a)
  }
  class Wrapper[A](val a: A)
  implicit def wrap[A](a: A): Wrapper[A] = new Wrapper(a)
}
import scala.annotation.nowarn

object Test {
  import Lib._
  def local(): String = {
    implicit val showDouble: Show[Double] = new Show[Double] { def show(d: Double) = s"D$d" }
    implicit def showOpt[A](implicit s: Show[A]): Show[Option[A]] = new Show[Option[A]] {
      def show(o: Option[A]) = o.fold("none")(s.show)
    }
    show(1.5) + " " + show(Option(2)) + " " + show(Option.empty[Int])
  }
  @nowarn def positional(): Unit = {
    println(show(5)(Show.showInt))
    println(show(6)(new Show[Int] { def show(a: Int) = "custom" }))
    val explicit: Show[List[Int]] = Show.showList(Show.showInt)
    println(explicit.show(List(3)))
  }
  def main(args: Array[String]): Unit = {
    println(show(1))
    println(show(List(1, 2)))
    println(show(true))
    println(show("s"))
    println(show(List(List(true))))
    println(max(3, 7))
    println(describe(4))
    println(summon[Ord[Int]].lt(1, 2))
    println(implicitly[Show[Int]].show(9))
    println(show(7)(using Show.showInt))
    positional()
    println(local())
    val byName = Show.showList[Int]
    println(byName.show(List(4)))
  }
}
