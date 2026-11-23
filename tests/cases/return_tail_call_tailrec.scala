// A def that carries `@tailrec` and calls itself under a `return`: the JVM and the interpreter accept it and loop, a
// million deep, as scalac does. On JavaScript, where such a call stays a call, `@tailrec` refuses it
// (tests/errors/tailrec_return_js; teq's row: Scala.js accepts and loops), so the program is pending there.
import scala.annotation.tailrec
object Main {
  @tailrec final def down(n: Int): Int = { if n > 0 then return down(n - 1); 0 }
  @tailrec final def sum(n: Int, acc: Long): Long = { if n == 0 then return acc; if n > 0 then return sum(n - 1, acc + n); -1L }
  def main(args: Array[String]): Unit = {
    println(down(1000000))
    println(sum(1000000, 0L))
  }
}
