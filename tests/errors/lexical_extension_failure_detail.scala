// expect: error: no given instance of type Second was found
// expect: 1 error found
// Where no imported extension's prefix holds and the implicit scope has none, the error is the
// first alternative's, as dotty commits `failures.head` and keeps its error: the innermost
// import's `B.pick`, whose `Second` has no instance, rather than a later alternative's
// failure.
trait First
trait Second
class R
object A:
  extension (r: R)(using First) def pick: Int = 1
object B:
  extension (r: R)(using Second) def pick: Int = 2
object Main:
  import A.*
  import B.*
  def main(args: Array[String]): Unit = println((new R).pick)
