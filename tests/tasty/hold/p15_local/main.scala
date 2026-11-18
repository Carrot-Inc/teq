import scala.language.implicitConversions
trait Ord[A]
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
object Main:
  def main(args: Array[String]): Unit =
    implicit def fallback(a: Int): Fallback = new Fallback(a)
    def run(): Unit =
      implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
      println(3.m(4))
      println(new Ops("s").m("t"))
    class Ops[A](a: A):
      def m(b: A) = List(a, b).head
    run()
