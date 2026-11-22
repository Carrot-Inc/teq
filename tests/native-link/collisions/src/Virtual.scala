package collide.virtual
abstract class Base {
  def Aa(): Int
  def BB(): Int
  def AaAa(): Int
  def AaBB(): Int
  def BBAa(): Int
  def BBBB(): Int
}
class One extends Base {
  def Aa(): Int = 0
  def BB(): Int = 1
  def AaAa(): Int = 2
  def AaBB(): Int = 3
  def BBAa(): Int = 4
  def BBBB(): Int = 5
}
class Two extends Base {
  def Aa(): Int = 10
  def BB(): Int = 11
  def AaAa(): Int = 12
  def AaBB(): Int = 13
  def BBAa(): Int = 14
  def BBBB(): Int = 15
}
object Main {
  def main(args: Array[String]): Unit = {
    val b: Base = if (args.length == 0) new One else new Two
    println(b.BBAa())
    println(b.Aa())
    println(b.BBBB())
    later(b)
  }
  def later(b: Base): Unit = {
    println(b.AaBB())
    println(b.BB())
    println(b.AaAa())
  }
}
