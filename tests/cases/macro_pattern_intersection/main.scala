object Main:
  val settings: Layer[Any, Nothing, Long] = new Layer("s")
  val greeter: Layer[Boolean, Throwable, Int] = new Layer("g")
  def main(args: Array[String]): Unit =
    println(Macros.describe(settings))
    println(Macros.describe(greeter))
