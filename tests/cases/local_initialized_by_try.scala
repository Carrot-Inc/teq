object Main {
  def read(fail: Boolean): Int = {
    val result = try {
      if fail then throw new IllegalStateException()
      val x = 8
      x + 1
    } catch { case _: IllegalStateException => 4 } finally { println("finally") }
    result + 1
  }
  def main(args: Array[String]): Unit = {
    println(read(false))
    println(read(true))
  }
}
