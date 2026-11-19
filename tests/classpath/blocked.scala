// jars: scala-library
// Misuses of loaded signatures, reported with the signatures.
// expect: 10:26: error: Found:    5
// expect: Required: Int => Any
// expect: 11:13: error: None of the overloaded alternatives of method abs in object package with types
// expect:  (x: Double): Double
import scala.concurrent.duration.DurationInt
object Main:
  def main(args: Array[String]): Unit =
    val ys = List(1).map(5)
    val n = math.abs("x")
