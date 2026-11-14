object Test:
  def main(args: Array[String]): Unit =
    println(Macros.created(3))
    println(Macros.direct(3))
