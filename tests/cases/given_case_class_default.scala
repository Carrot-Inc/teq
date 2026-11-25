// A case class of a given's body whose default reads the given's val, made through its companion and its constructor.
trait T
given G: T with {
 val base = 21
 case class K(x: Int = base)
 def made = K()
}
object Main { def main(args: Array[String]): Unit = { println(G.made.x); println(new G.K().x) } }
