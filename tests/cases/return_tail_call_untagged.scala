// The def of return_tail_call_tailrec without `@tailrec`: accepted on every target. The JVM and the interpreter loop;
// on JavaScript the call under the `return` stays a call, so the run here is shallow.
object Main {
  final def down(n: Int): Int = { if n > 0 then return down(n - 1); 0 }
  final def sum(n: Int, acc: Long): Long = { if n == 0 then return acc; if n > 0 then return sum(n - 1, acc + n); -1L }
  def main(args: Array[String]): Unit = {
    println(down(1000))
    println(sum(1000, 0L))
  }
}
