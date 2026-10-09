//> using scala 3.8.4
trait TC[A] { def s: String }
class Dog
object Dog { given [X >: Dog]: TC[X] = new TC[X] { def s = "dog-lower" } }
trait Holder:
  type T >: Dog
  def probe: String = summon[TC[T]].s
object Main extends Holder:
  type T = Any
  def main(args: Array[String]): Unit =
    println(probe)
