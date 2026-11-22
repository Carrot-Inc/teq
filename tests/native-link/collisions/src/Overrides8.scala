package collide.overrides8
abstract class Base {
  def mflz(): Int
  def mgMz(): Int
}
class One extends Base {
  def mflz(): Int = 1
  def mgMz(): Int = 2
}
class Two extends Base {
  def mflz(): Int = 3
  def mgMz(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.mflz()
}
class Second extends Step {
  def run(b: Base): Int = b.mgMz()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
