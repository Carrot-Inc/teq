// A local given conversion applies to the expressions of its block.
import scala.language.implicitConversions

case class W(i: Int)

object Main:
  def main(args: Array[String]): Unit =
    given Conversion[Int, W] = W(_)
    val w: W = 3
    println(w)
