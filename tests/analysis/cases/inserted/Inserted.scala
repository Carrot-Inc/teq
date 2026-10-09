package ins

import scala.language.implicitConversions

case class Celsius(v: Double)
object Celsius:
  given Conversion[Double, Celsius] = Celsius(_)

trait Codec[A]:
  def encode(a: A): String
object Codec:
  given Codec[Int] = (i: Int) => i.toString()
  given [A](using c: Codec[A]): Codec[List[A]] = (as: List[A]) => as.map(c.encode).mkString(",")

// Arguments and conversions the typer inserts, and the trees scalac's typer makes of casts,
// throws and conversions to strings.
object Inserted:
  def encode[A](a: A)(using c: Codec[A]): String = c.encode(a)
  val all: String = encode(List(1, 2))
  val warm: Celsius = 21.5
  val zero: Int = null.asInstanceOf[Int]
  val widened: Long = 3
  def fail(n: Int): Nothing = throw new IllegalStateException("no " + n.toString())
  def safe(n: Int): Int = try fail(n) catch { case _: IllegalStateException => 0 }
