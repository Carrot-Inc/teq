class V(val text: String) extends AnyVal

object Main:
  def make(s: String): V = new V(s)

  def main(args: Array[String]): Unit =
    println(make("abc")
      .text
      .length())
