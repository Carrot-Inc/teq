package collide.overrides10
abstract class Base {
  def plSc(): Int
  def pkrc(): Int
}
class One extends Base {
  def plSc(): Int = 1
  def pkrc(): Int = 2
}
class Two extends Base {
  def plSc(): Int = 3
  def pkrc(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.plSc()
}
class Second extends Step {
  def run(b: Base): Int = b.pkrc()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
