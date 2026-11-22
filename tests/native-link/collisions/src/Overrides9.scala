package collide.overrides9
abstract class Base {
  def Re(): Int
  def SF(): Int
}
class One extends Base {
  def Re(): Int = 1
  def SF(): Int = 2
}
class Two extends Base {
  def Re(): Int = 3
  def SF(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.Re()
}
class Second extends Step {
  def run(b: Base): Int = b.SF()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
