object Main {
  def matches(input: Seq[Any]): Boolean = input match {
    case Seq(a, b, rest*) =>
      println(rest.size)
      a == b
    case _ => false
  }
  def main(args: Array[String]): Unit = {
    println(matches(List(2, 2L)))
    println(matches(List("a", "b")))
    println(matches(List(2, 2L, "tail")))
    println(matches(Nil))
  }
}
