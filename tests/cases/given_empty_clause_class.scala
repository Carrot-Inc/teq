// A given with an empty clause (`given g: () => T`) is a class: its class made and read through it.
trait T { def made: Int }
given g: () => T { class K(val n: Int) { def get = n }; def made = new K(21).get }
object Main { def main(args: Array[String]): Unit = { println(g.made) } }
