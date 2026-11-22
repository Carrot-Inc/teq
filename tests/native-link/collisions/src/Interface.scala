package collide.interface
trait Shape {
  def Aa(): Int
  def BB(): Int
  def AaAa(): Int
  def AaBB(): Int
  def BBAa(): Int
  def BBBB(): Int
}
class Square extends Shape {
  def Aa(): Int = 0
  def BB(): Int = 1
  def AaAa(): Int = 2
  def AaBB(): Int = 3
  def BBAa(): Int = 4
  def BBBB(): Int = 5
}
class Circle extends Shape {
  def Aa(): Int = 20
  def BB(): Int = 21
  def AaAa(): Int = 22
  def AaBB(): Int = 23
  def BBAa(): Int = 24
  def BBBB(): Int = 25
}
object Main {
  def main(args: Array[String]): Unit = {
    val s: Shape = if (args.length == 0) new Square else new Circle
    println(s.AaAa())
    println(s.AaBB())
    println(s.BB())
    later(s)
  }
  def later(s: Shape): Unit = {
    println(s.BBBB())
    println(s.BBAa())
    println(s.Aa())
  }
}
