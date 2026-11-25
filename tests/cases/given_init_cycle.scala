// A class of a given made while the given initialises reads a val set before it and one not set yet (0).
trait T
given G: T with {
  val before = 7
  val first = new K().get
  val after = 21
  class K { def get: Int = before + after }
}
object Main { def main(args: Array[String]): Unit = { println(G.first); println(new G.K().get) } }
