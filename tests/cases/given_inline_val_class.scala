// A class of an inline given named through a val holding the given.
trait T
inline given G: T with { class K(val n: Int) }
object Main { def main(args: Array[String]): Unit = { val gg = G; println(new gg.K(21).n) } }
