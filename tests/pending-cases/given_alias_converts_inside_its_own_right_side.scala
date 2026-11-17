// A given alias is no conversion of a selection inside its own right-hand side: `q += 1` there is the
// assignment of the var (scalac 3.8.4). teq takes the given being defined as the conversion of `q` (master
// the same on `q.+=(1)`): the output throws on JavaScript.
import scala.language.implicitConversions
class R(val i: Int) { def +=(r: Int): Unit = println("R.+= " + i + " " + r) }
given conv: Conversion[Int, R] = { var q: Int = 1; q += 1; println("in given " + q); new R(_) }
object Main {
  def main(args: Array[String]): Unit = {
    val r: R = 5
    r += 2
  }
}
