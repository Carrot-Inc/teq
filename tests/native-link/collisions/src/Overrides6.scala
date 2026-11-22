package collide.overrides6
abstract class Base {
  def KwVJf(): Int
  def KwVKG(): Int
}
class One extends Base {
  def KwVJf(): Int = 1
  def KwVKG(): Int = 2
}
class Two extends Base {
  def KwVJf(): Int = 3
  def KwVKG(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.KwVJf()
}
class Second extends Step {
  def run(b: Base): Int = b.KwVKG()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
