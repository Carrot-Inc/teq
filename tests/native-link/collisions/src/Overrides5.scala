package collide.overrides5
abstract class Base {
  def ucSm(): Int
  def vDSm(): Int
}
class One extends Base {
  def ucSm(): Int = 1
  def vDSm(): Int = 2
}
class Two extends Base {
  def ucSm(): Int = 3
  def vDSm(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.ucSm()
}
class Second extends Step {
  def run(b: Base): Int = b.vDSm()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
