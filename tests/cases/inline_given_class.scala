// An inline given makes an instance at each read: its val's initialiser runs at each.
trait T { def madeInside: Int }
inline given G: T with {
  val n = { print("I"); 21 }
  class K { def get: Int = n }
  def madeInside: Int = new K().get
}
object Main { def main(args: Array[String]): Unit = { println(G.madeInside); println(G.madeInside) } }
