// scala.runtime's AbstractFunctionN, the base of a Scala 2 case class companion as zio's
// `Cause.Die` has it.
final case class Die(message: String, depth: Int)
object DieCompanion extends scala.runtime.AbstractFunction2[String, Int, Die]:
  def apply(message: String, depth: Int): Die = Die(message, depth)

object Zero extends scala.runtime.AbstractFunction0[String]:
  def apply(): String = "zero"

object Main:
  def main(args: Array[String]): Unit =
    val f: (String, Int) => Die = DieCompanion
    println(f("a", 1))
    println(List(("b", 2), ("c", 3)).map(DieCompanion(_, _)))
    println(Zero())
    println(DieCompanion.isInstanceOf[Function2[?, ?, ?]])
