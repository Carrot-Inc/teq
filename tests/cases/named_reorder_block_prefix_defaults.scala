case class C(x: Int, y: Int = 0)
object C:
  println("init")

def mark(s: String): Int =
  println(s)
  0

@main def run(): Unit =
  val c = ({ val z = 0; C }).apply(
    y = mark("y"), x = mark("x"))
