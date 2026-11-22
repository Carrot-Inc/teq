package collide.overrides0
abstract class Base {
  def Aa(): Int
  def BB(): Int
}
class One extends Base {
  def Aa(): Int = 1
  def BB(): Int = 2
}
class Two extends Base {
  def Aa(): Int = 3
  def BB(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.Aa()
}
class Second extends Step {
  def run(b: Base): Int = b.BB()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
