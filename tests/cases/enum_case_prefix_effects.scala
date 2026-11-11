enum E:
  case D(a: Int = 1, b: Int = 2)
object Main:
  lazy val delayed = { println("init"); E.D }
  def next = { println("call"); E.D }
  inline def inlined = { println("inline"); E.D }
  def which(x: E): String = "E"
  def which(x: E.D): String = "D"
  def main(args: Array[String]): Unit =
    println(which(delayed.apply(b = 3)))
    println(which(delayed.apply(b = 4)))
    println(which(next.apply(b = 3)))
    println(which(({ println("block"); E.D }).apply(b = 3)))
    println(which(inlined.apply(b = 3)))
