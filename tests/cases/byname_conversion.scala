// A Scala 2 implicit conversion with a by-name parameter takes the receiver as a thunk (cats'
// `catsSyntaxApplicativeByName(fa: => F[A])`), and evaluates it where the ops do.
import scala.language.implicitConversions
final class ByNameOps[A](fa: () => Option[A]):
  def whenA(cond: Boolean): Option[Unit] = if cond then fa().map(_ => ()) else Some(())
  def twice: List[Option[A]] = List(fa(), fa())
implicit def toByNameOps[A](fa: => Option[A]): ByNameOps[A] = new ByNameOps(() => fa)
object Main:
  var evaluated = 0
  def compute: Option[Int] =
    evaluated += 1
    Some(evaluated)
  def main(args: Array[String]): Unit =
    println(Option(1).whenA(true))
    println(compute.whenA(false))
    println(evaluated)
    println(compute.twice)
    println(evaluated)
