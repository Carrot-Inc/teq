// A `@tailrec` method whose only recursive call stands in a branch the constant fold drops has no recursive call.
// expect: 4:33: error: TailRec optimisation not applicable, method run contains no recursive calls
object Main {
  @scala.annotation.tailrec def run(n: Int): Int = if (false) run(n - 1) else n
  def main(args: Array[String]): Unit = println(run(args.length))
}
