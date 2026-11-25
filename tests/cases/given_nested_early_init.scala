// A class of a given nested in a given, made while the inner given initialises, reads the val set before it.
trait T
given H: T with {
 given G: T with {
  val early = 7
  class K { def get: Int = early }
  val first = new K().get
 }
}
object Main { def main(args: Array[String]): Unit = println(H.G.first) }
