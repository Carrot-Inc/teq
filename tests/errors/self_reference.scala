// expect: 6:9: error: recursive value b needs type
// expect: 1 error found
object T:
  val b = 42
  def main(): Unit =
    val b = b + 1
    println(b)
  def fine(): Unit =
    val f: Int => Int = x => if x == 0 then 1 else f(x - 1)
    println(f(3))

@main def run(): Unit = T.main()
