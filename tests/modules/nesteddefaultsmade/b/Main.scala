package ndm

object Main {
  def main(args: Array[String]): Unit = {
    val o = new Outer
    println(o.Item().hidden)
  }
}
