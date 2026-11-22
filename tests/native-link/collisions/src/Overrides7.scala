package collide.overrides7
abstract class Base {
  def wj(): Int
  def xK(): Int
}
class One extends Base {
  def wj(): Int = 1
  def xK(): Int = 2
}
class Two extends Base {
  def wj(): Int = 3
  def xK(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.wj()
}
class Second extends Step {
  def run(b: Base): Int = b.xK()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
