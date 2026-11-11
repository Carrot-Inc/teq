// ImportInfo.importedImplicits keeps the import's site for each implicit reference, and a renamed selector's
// RenamedImplicitRef: the same inherited `wrap`, imported from two objects under two names, is two candidates,
// each seen from its own object.
import scala.language.implicitConversions

final class Box[A](val value: A)
trait Conversions[A]:
  implicit def wrap(x: A): Box[A] = new Box(x)
object IntConversions extends Conversions[Int]
object StringConversions extends Conversions[String]

object Use:
  import IntConversions.{wrap as intWrap}
  import StringConversions.{wrap as stringWrap}
  val i: Box[Int] = 1
  val s: Box[String] = "s"

@main def run(): Unit =
  println(Use.i.value + 1)
  println(Use.s.value + "!")
