object P {
  var a = 0
  var b = 0
  val t = (a = 1, b = 2)
  val u = (a = 3)
  def main(args: Array[String]): Unit = { println(t); println((u, 0)); println((a, b)) }
}
