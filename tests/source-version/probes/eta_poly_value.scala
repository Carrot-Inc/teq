object P {
  val p = [A] => (a: A) => a
  val g = p _
  def main(args: Array[String]): Unit = println(g[Int](1) + g("xy").length)
}
