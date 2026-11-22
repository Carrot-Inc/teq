package collide.overrides1
abstract class Base {
  def AaAa(): Int
  def BBBB(): Int
}
class One extends Base {
  def AaAa(): Int = 1
  def BBBB(): Int = 2
}
class Two extends Base {
  def AaAa(): Int = 3
  def BBBB(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.AaAa()
}
class Second extends Step {
  def run(b: Base): Int = b.BBBB()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
