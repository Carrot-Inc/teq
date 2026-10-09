// jars: scala-library
// Misuses of loaded signatures, reported with the signatures.
// expect: 9:26: error: type mismatch: found Int, required Int => B
// expect: 10:13: error: None of the overloaded alternatives of method abs in object package with types
// expect:  (x: Double): Double
import scala.concurrent.duration.DurationInt
object Main:
  def main(args: Array[String]): Unit =
    val ys = List(1).map(5)
    val n = math.abs("x")
