object Prefix:
  println("init")
  extension (right: String)
    def ++:(left: String)(result: String = left + right): String = result
object Main:
  def main(args: Array[String]): Unit =
    println(Prefix.++:("a")("b")())
    println(Prefix.++:("c")("d")())
