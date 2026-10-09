//> using file lib/chain/Chained.scala
// An included file's own directives are followed, which Scala CLI 1.17.1 does not do ("Chaining the
// 'using file' directive is not supported"): its expectation is teq's, written by hand.
object Main:
  def main(args: Array[String]): Unit = println(Chained.value)
