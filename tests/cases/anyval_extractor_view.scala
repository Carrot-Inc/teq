// std: scala-library
// teq: --std=scala-library
// Where no alternative applies to an `Any` scrutinee, the overload resolution's second round
// takes views (`resolveOverloaded`'s implicit-enabled pass, `isCompatible`'s `viewExists`):
// Predef's value classes (`Ensuring`) view any value as an `AnyVal`, none views it as a
// `String` or an `AnyRef`, so the `AnyVal` extractor is chosen and its typing narrows the
// scrutinee. An alternative that applies in the first round (`Any`, `Matchable`) wins as
// before.
import scala.annotation.targetName

object E:
  def unapply(x: String): Option[String] = Some("string")
  def unapply(x: AnyVal): Option[String] = Some("val")
object N:
  @targetName("unapplyVal") def unapply(x: AnyVal): Option[Int] = Some(1)
  def unapply(x: AnyRef): Option[String] = Some("ref")
object T:
  def unapply(x: Any): Option[String] = Some("any")
  @targetName("unapplyVal") def unapply(x: AnyVal): Option[String] = Some("val")
object M:
  def f(x: String): String = "string"
  def f(x: Matchable): String = "matchable"

def narrow(x: Any): String = x match { case N(s) => s.toString; case _ => "none" }

@main def run(): Unit =
  val x: Any = 1
  println(x match { case E(s) => s; case _ => "none" })
  println(narrow(1))
  println(narrow("s"))
  println(x match { case T(s) => s; case _ => "none" })
  println(T.unapply(x))
  println(M.f(x))
