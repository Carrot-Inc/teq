package collide.overrides4
abstract class Base {
  def DnNgSi(): Int
  def DnNfri(): Int
}
class One extends Base {
  def DnNgSi(): Int = 1
  def DnNfri(): Int = 2
}
class Two extends Base {
  def DnNgSi(): Int = 3
  def DnNfri(): Int = 4
}
abstract class Step {
  def run(b: Base): Int
}
class First extends Step {
  def run(b: Base): Int = b.DnNgSi()
}
class Second extends Step {
  def run(b: Base): Int = b.DnNfri()
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Step = if (args.length == 0) new First else new Second
    val b: Base = if (args.length == 1) new One else new Two
    println(s.run(b))
  }
}
