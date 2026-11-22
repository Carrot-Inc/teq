package collide.overrides3
abstract class Base {
  def BBBB(): Int
  def AaAa(): Int
}
class One extends Base {
  def BBBB(): Int = 1
  def AaAa(): Int = 2
}
class Two extends Base {
  def BBBB(): Int = 3
  def AaAa(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.BBBB()
}
class Second extends Step {
  def run(b: Base): Int = b.AaAa()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
