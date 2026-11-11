enum E:
  case D(a: Int = 1)
object Main:
  def which(x: E): String = "E"
  def which(x: E.D): String = "D"
  def main(args: Array[String]): Unit =
    println(which((E.D: E.D.type).apply()))
