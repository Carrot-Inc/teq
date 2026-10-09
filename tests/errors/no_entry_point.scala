// A build writes a program; one without an entry point or an export is rejected rather than
// written empty. A `main` with another signature is no entry point.
// command: build
// expect: the program has no entry point
object Lib:
  def main(args: List[String]): Unit = println(args)
  def twice(n: Int): Int = n * 2
