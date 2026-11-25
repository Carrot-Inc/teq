// A class of a nested given made during its initialisation reads a later val, not yet set (0).
trait T
given H: T with {
 val before = 7
 given G: T with {
   val first = new K().get
   val after = 21
   class K { def get: Int = if after == 0 then before else after }
 }
}
object Main { def main(args: Array[String]): Unit = { println(H.G.first); println(new H.G.K().get) } }
