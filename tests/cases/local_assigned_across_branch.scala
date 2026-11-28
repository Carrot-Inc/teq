object Main {
  def choose(b: Boolean): Int = {
    var x = 3
    if b then x += 2 else x *= 2
    x
  }
  def main(args: Array[String]): Unit = {
    println(choose(true))
    println(choose(false))
  }
}
