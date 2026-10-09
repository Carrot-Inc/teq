// The NotGiven fallback pattern against scala-library's companion (its amb1/amb2/default are no candidates: scalac negates every candidate's result)
//> using scala 3.8.4
import scala.util.NotGiven
trait Show[A]
given Show[Int] = new Show[Int] {}
trait TC[A] { def s: String }
object TC:
  given low[T](using NotGiven[Show[T]]): TC[T] = new TC[T] { def s = "no-show" }
  given high[T](using Show[T]): TC[T] = new TC[T] { def s = "show" }
object Main:
  def main(args: Array[String]): Unit =
    println(summon[TC[Int]].s)
    println(summon[TC[String]].s)
