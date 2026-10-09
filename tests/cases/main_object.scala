// The JVM entry point: main(args: Array[String]) of a top-level object, with an empty args array.
object Helper:
  def greet(name: String): String = s"hello $name"

object Test:
  val setup = { println("object body first"); 1 }
  def main(args: Array[String]): Unit =
    println(s"args: ${args.length}")
    println(Helper.greet("main"))
    println(args.mkString("[", ",", "]"))
    println(setup)
