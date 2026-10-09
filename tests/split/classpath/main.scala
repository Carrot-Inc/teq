package app

object Main {
  def main(args: Array[String]): Unit = {
    val xs = List(3, 1, 2)
    println(util.Fmt.show(xs.sorted))
  }
}
