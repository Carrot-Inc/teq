//> using scala 3.8.4
import scala.util.NotGiven

// NotGiven[T] is provided exactly when the search for T finds nothing.
trait Show[A] { def n: String }
given showInt: Show[Int] with { def n = "int" }

def describe[A](using NotGiven[Show[A]]): String = "no show"
def describe2[A](using Show[A]): String = "show"

trait Tag[A] { def n: String }
object Tag:
  given shown[A](using s: Show[A]): Tag[A] with { def n = "shown as " + s.n }
  given unshown[A](using NotGiven[Show[A]]): Tag[A] with { def n = "unshown" }

def tag[A](using t: Tag[A]): String = t.n

class Boxed
given boxedShow: Show[Boxed] with { def n = "boxed" }

@main def main(): Unit =
  println(describe[String])
  println(describe2[Int])
  println(tag[Int])
  println(tag[String])
  println(tag[Boxed])
  val ev: NotGiven[Show[Boolean]] = summon[NotGiven[Show[Boolean]]]
  println(ev.isInstanceOf[NotGiven[?]])
