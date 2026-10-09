// The expected type constrains an application's result only once its last argument list is
// reached, so the earlier lists' arguments settle the type argument and the whole result
// converts; a lambda or an eta-expanded method against the constrained variable converts in
// its body instead, as scalac adapts each.
import scala.language.implicitConversions
final case class W(s: String):
  override def toString = s"W($s)"
object W:
  given Conversion[String, W] = W(_)
object Main:
  def rps(d: Double): String = d.toInt.toString
  def one[B](f: Double => B): B = f(1.0)
  def two[B](ifEmpty: B)(f: Double => B): B = f(2.0)
  def three[B](ifEmpty: B, f: Double => B): B = f(3.0)
  def main(args: Array[String]): Unit =
    val total: Option[Double] = Some(4.0)
    val none: Option[Double] = None
    val a: W = two("-")(rps)
    val b: W = one(rps)
    val c: W = three("-", x => W("l" + x.toInt))
    val d: W = two("q")(x => "s" + x.toInt)
    val e: W = two(W("q"))(x => "t" + x.toInt)
    val f: W = one(x => "u" + x.toInt)
    val g: W = total.fold("-")(rps)
    val h: W = none.fold("-")(rps)
    val i: W = three("-", rps)
    println(List(a, b, c, d, e, f, g, h, i).mkString(" "))
