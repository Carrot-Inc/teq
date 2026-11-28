object Main {
  def left(): Any = { println("left"); "value" }
  def compare(fail: Boolean): Boolean =
    left() == (try {
      if fail then throw new IllegalStateException()
      "value"
    } catch { case _: IllegalStateException => "caught" })
  def main(args: Array[String]): Unit = {
    println(compare(false))
    println(compare(true))
  }
}
