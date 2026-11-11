class O:
  enum F:
    case K(n: Int)
    case D(a: Int = 1, b: Int = 2)
  def dfd = F.D

enum J:
  case K(n: Int)
  case D(a: Int = 1, b: Int = 2)

object Main:
  val a = new O
  def w(x: a.F): String = "F"
  def w(x: a.F.K): String = "K"
  def w(x: a.F.D): String = "D"
  def wj(x: J): String = "J"
  def wj(x: J.K): String = "K"
  def wj(x: J.D): String = "D"
  def z(): Int = 0
  def dod = a.F.D
  def djd = J.D
  def main(args: Array[String]): Unit =
    println(List(
      w(dod.apply()), w(dod.apply(b = 3)), w(a.dfd.apply()), w(({ println("x"); a.F.D }).apply()), w(dod.apply(z(), 1)),
      wj(djd.apply()), wj(djd.apply(b = 3)), wj(({ println("y"); J.D }).apply()), wj(djd.apply(z(), 1)), wj(djd(z(), 1))
    ).mkString(" "))
