// Conversions to function types, sequences, a local implicit def, an object that mixes in a trait of conversions, and a receiver evaluated once.
import scala.language.implicitConversions
class Cell(var v: Int) { def bump(): Cell = { v += 1; this }; def get: Int = v }
trait Conv { implicit def toCell(i: Int): Cell = new Cell(i) }
object ConvInstances extends Conv
class User {
  def run(): String = {
    import ConvInstances._
    val c = 4.bump().bump()
    s"${c.get} ${(10: Cell).get}"
  }
}
object Fn {
  class Rep(val s: String)
  implicit def toFn(r: Rep): Int => String = i => r.s * i
  implicit def toSeq(n: Int): List[Int] = List.fill(n)(n)
}
object Local {
  def go(): String = {
    implicit def local(s: String): Cell = new Cell(s.length)
    "abcd".bump().get.toString
  }
}
object Test {
  def main(args: Array[String]): Unit = {
    println(new User().run())
    import Fn._
    println(new Rep("ab")(3))
    println(3.map(_ + 1))
    println(2.sum)
    println(Local.go())
    var count = 0
    def next(): Int = { count += 1; count }
    import ConvInstances._
    println((next(): Cell).get + " " + count)
    val xs: List[Int] = 2
    println(xs)
    def twice(f: Int => String): String = f(2)
    println(twice(new Rep("xy")))
  }
}
