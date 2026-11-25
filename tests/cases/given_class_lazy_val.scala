// A class of a given reads the given's lazy val: initialised once, at the first read.
trait T
given G: T with {
  lazy val base = { print("L"); 21 }
  class K { def get: Int = base }
}
object Main { def main(args: Array[String]): Unit = { val k = new G.K; print("|"); println(k.get); println(k.get) } }
