package dirs

object Main {
  def main(args: Array[String]): Unit =
    println(A.value(args.length) + B.value + C.value)
}
