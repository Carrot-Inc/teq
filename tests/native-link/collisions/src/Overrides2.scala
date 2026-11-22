package collide.overrides2
abstract class Base {
  def AaBB(): Int
  def BBAa(): Int
}
class One extends Base {
  def AaBB(): Int = 1
  def BBAa(): Int = 2
}
class Two extends Base {
  def AaBB(): Int = 3
  def BBAa(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.AaBB()
}
class Second extends Step {
  def run(b: Base): Int = b.BBAa()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
