package collide.overrides11
abstract class Base {
  def fl(): Int
  def gM(): Int
}
class One extends Base {
  def fl(): Int = 1
  def gM(): Int = 2
}
class Two extends Base {
  def fl(): Int = 3
  def gM(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.fl()
}
class Second extends Step {
  def run(b: Base): Int = b.gM()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
