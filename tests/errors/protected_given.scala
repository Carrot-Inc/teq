// expect: 9:30: error: no given instance of type Show[Int] was found for parameter x
// expect: 1 error found
trait Show[A]
object Show:
  protected given hidden: Show[Int] = new Show[Int] {}
import Show.given
object Main:
  def main(args: Array[String]): Unit =
    println(summon[Show[Int]])
