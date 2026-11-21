// A program for the NIR reader's fixtures: compiled by Scala Native's own compiler plugin, it puts
// most of what a NIR file can hold into a few files (tests/nir/README).
package fix

import scala.scalanative.annotation.{align, alwaysinline, nooptimize, nospecialize}
import scala.scalanative.meta.LinktimeInfo
import scala.scalanative.unsafe.*

trait Shape:
  def area: Double
  def name: String = "shape"

abstract class Named(val label: String) extends Shape:
  override def name: String = label

final case class Circle(r: Double) extends Named("circle"):
  def area: Double = math.Pi * r * r

final case class Rect(w: Double, h: Double) extends Named("rect"):
  def area: Double = w * h

enum Colour:
  case Red, Green
  case Mixed(parts: Int)

@extern
@link("m")
@define("FIX_SHAPES")
object libm:
  def cbrt(x: CDouble): CDouble = extern
  @name("printf")
  def print(format: CString, args: Any*): CInt = extern
  @blocking def usleep(micros: CUnsignedInt): CInt = extern

final class Counters:
  @volatile var seen: Long = 0L
  @align(64) var padded: Int = 0
  lazy val table: Array[Int] = Array.tabulate(8)(i => i * i)

object Shapes:
  type Pair = CStruct2[CInt, CDouble]

  val texts: List[String] =
    List("plain", "quo\"te", "back\\slash", "line\nfeed", "\nfirst", "two\n\nfeeds", "tab\t", "é", "😀", "\\\"")
  val numbers: Array[Double] =
    Array(0.0, -0.0, 1.5, 1e-7, 1e21, Double.NaN, Double.PositiveInfinity, Double.MinPositiveValue, Double.MaxValue)
  val floats: Array[Float] = Array(0.1f, -3.4028235e38f, Float.NaN, 1.4e-45f)
  val longs: Array[Long] = Array(Long.MinValue, -1L, 0L, Long.MaxValue)
  val chars: Array[Char] = Array('a', '\u0000', '￿', '"')

  @alwaysinline def twice(x: Int): Int = x * 2
  @noinline def describe(s: Shape): String = s match
    case Circle(r) if r > 1.0 => "big circle"
    case c: Circle => "circle " + c.r
    case Rect(w, h) => s"rect $w by $h"
    case other => other.name

  @nooptimize def classify(n: Int): String = n match
    case 0 => "zero"
    case 1 | 2 => "small"
    case 100 => "hundred"
    case _ => "other"

  @nospecialize def word(s: String): Int = s match
    case "one" => 1
    case "two" => 2
    case _ => -1

  def guarded(xs: Array[Int], i: Int): Int =
    try xs(i) / xs.length
    catch
      case _: ArithmeticException => -1
      case e: ArrayIndexOutOfBoundsException => throw new IllegalStateException("at " + i, e)
    finally xs(0) = xs(0) + 1

  def locked(c: Counters): Long = c.synchronized {
    c.seen += 1
    c.seen
  }

  def native(n: Int): Double =
    val buffer = stackalloc[CInt](4)
    var i = 0
    while i < 4 do
      buffer(i) = n + i
      i += 1
    val pair = stackalloc[Pair]()
    pair._1 = !buffer
    pair._2 = libm.cbrt(27.0)
    libm.print(c"%d \"cubes\"\n\\ %f\n", pair._1, pair._2)
    sizeOf[Pair].toDouble + alignmentOf[CLong].toDouble + pair._2

  def platform(): String =
    if LinktimeInfo.isWindows then "windows"
    else if LinktimeInfo.isLinux || LinktimeInfo.isMac then "unix"
    else "other"

  def closures(xs: List[Int]): Int =
    val offset = xs.length
    val add: Int => Int = _ + offset
    xs.map(add).filter(_ % 2 == 0).foldLeft(0)(_ + _)

  def boxes(x: Any): Any = x match
    case i: Int => i + 1
    case l: Long => l + 1L
    case c: Char => c.toInt
    case b: Boolean => !b
    case u: Unit => u
    case null => "null"
    case _ => x

  def main(args: Array[String]): Unit =
    val shapes: List[Shape] = List(Circle(2.0), Rect(1.0, 2.0))
    shapes.foreach(s => println(describe(s) + " " + s.area))
    println(classify(args.length) + word("one") + guarded(Array(1, 2), 1) + locked(new Counters))
    println(native(3))
    println(platform() + closures(List(1, 2, 3)) + boxes(1) + Colour.Mixed(2).toString + twice(4))
    println(texts.mkString + numbers.sum + floats.sum + longs.sum + chars.mkString)
