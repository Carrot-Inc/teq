// `summonInline` and `summonFrom` in inline bodies, and a block import a body's search sees:
// scalac searches where teq's body expands, the givens of the call site and the body's import.
import scala.compiletime.{summonFrom, summonInline}

trait Show[A]:
  def show(a: A): String

object Instances:
  given Show[Int] = (a: Int) => s"int $a"
  given Show[String] = (a: String) => s"string $a"

object Shows:
  inline def show[A](a: A): String =
    import Instances.given
    summonInline[Show[A]].show(a)
  inline def maybe[A](a: A): String = summonFrom {
    case s: Show[A] => s.show(a)
    case _ => "no show"
  }

object InlineSummon:
  def main(args: Array[String]): Unit =
    println(Shows.show(1))
    println(Shows.show("a"))
    println(Shows.maybe(2.0))
    import Instances.given
    println(Shows.maybe(3))
