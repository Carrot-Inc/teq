package probe
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
object Collision {
  def main(args: Array[String]): Unit = {
    val b: Base = if (args.length == 0) new One else new Two
    println(b.BB())
    println(b.Aa())
  }
}
