// A @main method inside an object, taking the command line as String*.
object Runner:
  def count(xs: Seq[String]): Int = xs.length
  @main def run(args: String*): Unit =
    println(s"received ${count(args)} arguments")
    println(args.toList)
