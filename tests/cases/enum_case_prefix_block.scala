enum E:
  case D(a: Int = 1, b: Int = 2)
object Main:
  def which(x: E): String = "E"
  def which(x: E.D): String = "D"
  def main(args: Array[String]): Unit =
    println(which(({ import E.*; E.D }).apply(b = 3)))
    println(which(({ class C; E.D }).apply(b = 3)))
