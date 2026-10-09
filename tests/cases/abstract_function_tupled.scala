// A class that extends a function type has the function's `tupled` and `curried`, as a Scala 2
// case class companion (`object Die extends AbstractFunction2`) is used.
import scala.runtime.AbstractFunction2

final case class Die(message: String, depth: Int)
object DieCompanion extends AbstractFunction2[String, Int, Die]:
  def apply(message: String, depth: Int): Die = Die(message, depth)

class Add(base: Int) extends ((Int, Int, Int) => Int):
  def apply(a: Int, b: Int, c: Int): Int = base + a + b + c

object Main:
  def main(args: Array[String]): Unit =
    println(DieCompanion.tupled(("a", 1)))
    println(List(("b", 2), ("c", 3)).map(DieCompanion.tupled))
    println(DieCompanion.curried("d")(4))
    val add = new Add(10)
    println(add.tupled((1, 2, 3)))
    println(add.curried(1)(2)(3))
