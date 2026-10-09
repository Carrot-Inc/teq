package tma

// Two entry points in one module: its products, a JVM build's and a Scala.js check's, need
// none chosen.
object First:
  def greet: String = "first"
  def main(args: Array[String]): Unit = println(greet)

object Second:
  def main(args: Array[String]): Unit = println("second")
