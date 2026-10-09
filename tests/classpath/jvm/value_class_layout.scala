// jars: scala-library
// std: scala-library
// A value class in scalac's layout: its methods' bodies are its
// companion's `m$extension`, which the box and every call reach, a lambda inside one capturing the box as `this`;
// a value case class's erased family, and its pattern on the underlying value binding the field.
trait Showable extends Any { def show: String = s"<${raw}>"; def raw: String }
class Meters(val v: Double) extends AnyVal with Showable:
  def +(o: Meters): Meters = Meters(v + o.v)
  def scaled(k: Double = 2.0): Meters = Meters(v * k)
  def raw: String = v.toString
  def adders: List[Double => Double] = List(d => d + v, d => d * v)
  @annotation.tailrec final def halve(n: Int, acc: Meters = this): Meters = if n == 0 then acc else halve(n - 1, Meters(acc.v / 2))
  private def secret: Double = v * 10
  def reveal: Double = secret + 1
case class Id(value: Long) extends AnyVal:
  def next: Id = copy(value + 1)
  def label(prefix: String = "id"): String = s"$prefix-$value"
case class Box[T](t: T) extends AnyVal:
  def map[U](f: T => U): Box[U] = Box(f(t))
object Syntax:
  implicit class RichInt(val n: Int) extends AnyVal:
    def twice: Int = n * 2
object Main:
  import Syntax.*
  def main(args: Array[String]): Unit =
    val m = Meters(1.5)
    println((m + Meters(2.0)).v)
    println(m.scaled().v)
    println(m.scaled(3).v)
    println(m.show)
    println(m.adders.map(f => f(10)))
    println(m.halve(3).v)
    println(m.reveal)
    val id = Id(41)
    println(id.next)
    println(id.label())
    println(id.label("x"))
    id match { case Id(n) => println(n) }
    println(id.copy(7).value)
    println(id == Id(41))
    println(id.hashCode == Id(41).hashCode)
    println(Box(3).map(_ + 1).map(_.toString * 2))
    println(5.twice)

    val s: Showable = m
    println(s.show)
