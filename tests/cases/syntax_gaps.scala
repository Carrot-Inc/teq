import scala.language.implicitConversions
import scala.language.{postfixOps, higherKinds}
import scala.language.experimental.subCases
import scala.collection.mutable.ArrayBuffer

trait Show[A]:
  def show(a: A): String
object Show:
  given Show[Int]:
    def show(a: Int) = s"int $a"
  given Show[String]:
    def show(a: String) = s"str $a"

class Context(val name: String)
given Context("alpha")
class Counter(val start: Int, val step: Int)
given counter: Counter(10, 2)
object Fmt:
  given Show[Boolean]:
    def show(a: Boolean) = if a then "yes" else "no"

def describe[A: Show as s](a: A): String = s.show(a)
def both[A: {Show as sa}, B: {Show as sb}](a: A, b: B): String = sa.show(a) + "/" + sb.show(b)
class Box[A: Show as ev](a: A):
  def text = ev.show(a)

def sum(xs: Int*): Int = xs.sum

object Test:
  import Fmt.{given Show[Boolean]}
  def main(args: Array[String]): Unit =
    val a, b = ArrayBuffer(1)
    a += 2
    println(a)
    println(b)
    val x, y: Int = 3
    var i, j = 0
    i += 1
    println(x + y + i + j)
    println(summon[Context].name)
    println(counter.start + counter.step)
    println(describe(4))
    println(both(1, "one"))
    println(Box("s").text)
    println(describe(true))
    val xs = List(1, 2, 3)
    println(sum(xs: _*))
    val m = xs match { case List(1, rest: _*) => rest.size; case _ => 0 }
    println(m)
    val n = xs.size match { case 3 => "three"; case _ => "other" }
    println(n)
    val ys = xs map { x => x * 2 }
    println(ys)
    val zs = xs filter { _ > 1 } map { case n => n + 1 }
    println(zs)
    xs foreach { x =>
      println(x)
    }
    val ws = xs ++ { List(9) }
    println(ws)
