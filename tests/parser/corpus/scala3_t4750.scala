// Adapted from scala3 tests/run/t4750.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
import scala.util.matching.Regex

object Test {
  def main(args: Array[String]): Unit = ()
  val input = "CURRENCY 5.80"
  println("CURRENCY".r.replaceAllIn(input, Regex quoteReplacement "US$"))
}

