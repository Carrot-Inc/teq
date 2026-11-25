// The same in a given nested in a given: the outer given's val is set, the inner's later val not yet.
trait T
given H: T with {
  val base = 7
  given G: T with {
    val first = new K().get
    val after = 21
    class K { def get: Int = base + after }
  }
}
object Main { def main(args: Array[String]): Unit = { println(H.G.first); println(new H.G.K().get) } }
