final class Count(val value: Int) extends AnyVal {
  def advance(left: Int): Int = {
    println(value)
    if left == 0 then value else new Count(value + left).advance(left - 1)
  }
}
object Main {
  def main(args: Array[String]): Unit = println(new Count(10).advance(3))
}
