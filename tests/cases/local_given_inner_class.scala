// A class of a given local to a method reads the method's parameter: each call its own.
trait T
object Main { def main(args: Array[String]): Unit = { def run(n: Int): Int = { given G: T with { class K { def get = n } }; new G.K().get }; println(run(21)); println(run(42)) } }
