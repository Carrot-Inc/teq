object Main {
  def part(i: Int): Int = { println(i); i }
  def show(i: Int, ref: String, missing: String): Unit = {
    println(s"${part(i)}:$ref:$missing:${part(i + 1)}")
    println(raw"${part(i)}:$ref:$missing:${part(i + 1)}")
  }
  def main(args: Array[String]): Unit = show(4, "value", null)
}
