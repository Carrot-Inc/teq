object Main {
  def left(): Any = { println("left"); 7 }
  def compare(stop: Boolean): Boolean =
    left() == (if stop then return false else 7L)
  def main(args: Array[String]): Unit = {
    println(compare(true))
    println(compare(false))
  }
}
