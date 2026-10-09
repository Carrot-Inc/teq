object T:
  val a = 42
  def main(): Unit =
    val a: Int = a
    println(a)
  def block(): Int =
    val x = y
    val y = 1
    x
@main def run(): Unit = println(T.block())
