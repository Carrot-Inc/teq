// A value class with its explicit companion in a parameterless given, made through the companion.
trait T
given G: T with {
 class V(val x: Int) extends AnyVal { def twice: Int = x * 2 }
 object V { def make(x: Int): V = new V(x) }
}
object Main { def main(args: Array[String]): Unit = println(G.V.make(21).twice) }
