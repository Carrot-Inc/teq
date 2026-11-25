// `given g(): T with` (an empty clause in the old syntax) is a syntax error, and its use a given that takes no parameters.
// expect: 5:9: error: `using` expected
// expect: 6:63: error: given instance g does not take parameters
trait T
given g(): T with { class K(val n: Int) { def get = n }; def made = new K(21).get }
object Main { def main(args: Array[String]): Unit = { println(g().made) } }
