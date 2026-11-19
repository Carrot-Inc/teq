// expect: 9:14: error: No given instance of type NotGiven[Int] was found for parameter x$1 of method f in object Main
//> using scala 3.8.4
import scala.util.NotGiven
object Main:
  given Int = 1
  given nf: NotGiven[Int] = NotGiven.value
  def f(using NotGiven[Int]): String = "not given"
  def main(args: Array[String]): Unit =
    println(f)
