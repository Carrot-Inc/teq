// An implicit class's constructor default: the companion that holds its getter shares its term
// name with the class's conversion, and `new Applied[R]` reads the default on that companion.
object IcDefaults:
  implicit final class Applied[R](private val dummy: Boolean = true) extends AnyVal:
    def apply(x: Int): Int = if dummy then x + 1 else x
  def applied[R]: Applied[R] = new Applied[R]
  implicit class Counted(val n: Int = 3):
    def twice: Int = n * 2
  def counted: Counted = new Counted

@main def main(): Unit =
  println(IcDefaults.applied[String](1))
  println(IcDefaults.counted.twice)
  println(IcDefaults.Applied[Int](false)(1))
