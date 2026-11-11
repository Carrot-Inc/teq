enum E:
  case D(a: Int = 1, b: Int = 2)
object Ex:
  export E.D
object Main:
  val stable = E.D
  def which(x: E): String = "E"
  def which(x: E.D): String = "D"
  inline def inl = E.D
  def main(args: Array[String]): Unit =
    println(which((E.D: E.D.type).apply(b = 3)))
    println(which((stable: E.D.type).apply(b = 3)))
    println(which((Ex.D: E.D.type).apply(b = 3)))
    println(which(inl.apply(b = 3)))
    println(which(({ E.D }).apply(b = 3)))
