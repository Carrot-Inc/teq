object Main:
  def main(args: Array[String]): Unit =
    println(ElementLabel.of(List(1, 2)))
    println(ElementLabel.of(List("a")))
    println(ElementLabel.of(3))
