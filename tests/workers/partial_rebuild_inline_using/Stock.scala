package inc
trait Show[A]:
  def show(a: A): String
object Stock:
  given intShow: Show[Int] = new Show[Int] { def show(a: Int) = "i" + a }
  def render[A](a: A)(using s: Show[A]) = { val text = s.show(a); text + 1 }
