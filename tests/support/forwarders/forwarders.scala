// The static forwarders of a mirror class stand in scalac's order: by their source names, the
// object's own and the inherited ones alike, the overloads of a name by the erased types of
// their parameters (tests/run_jvm.sh compares the listing with forwarders.expected, which is
// javap's of scalac's class file).
package p
class Inner1
object Outer:
  class Nested
trait Described:
  def zeta: Int = 1
  def gen(x: Long, y: Long): Int = 2
class Base:
  def mid: Int = 3
  def gen(x: Char): Int = 4
object statusCode extends Base with Described:
  def z1: Int = 1
  def ~(x: Int): Int = x
  def +(x: Int): Int = x
  def Zed: Int = 1
  def a_b: Int = 1
  def aB: Int = 1
  def description: String = "d"
  def and(other: Int): Int = other
  def gen[T](x: T): T = x
  def gen(x: String): String = x
  def gen(x: Any, y: Int): Any = x
  def gen(x: Outer.Nested): Int = 1
  def gen(x: Inner1): Int = 1
  def gen(x: Array[Int]): Int = 1
  def gen(x: Array[String]): Int = 1
  def gen(x: List[Int]): Int = 1
  def gen(x: Int): Long = 1
  def gen(x: Unit): Int = 1
  def gen(): Int = 7
  def gen(b: Boolean, c: Char): Int = 8
  def gen(x: Double): Int = 5
  def res(x: Int): String = ""
@main def run(): Unit = println(statusCode.z1 + statusCode.zeta + statusCode.mid)
