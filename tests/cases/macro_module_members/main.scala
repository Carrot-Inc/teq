package mm

object Registry:
  def apply[T](a: Int, b: String): String = s"$a$b"
  def apply(x: Int): Int = x
  def other: Int = 1

@main def run(): Unit = println(mmacro.M.show)
