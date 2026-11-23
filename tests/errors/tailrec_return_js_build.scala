// command: build
// expect: 8:22: error: Cannot rewrite recursive call: it is not in tail position
// expect: 1 error found
// A JavaScript build refuses what its check refuses (tailrec_return_js): `@tailrec` on a def whose call of itself
// stands under a `return`.
import scala.annotation.tailrec
object Down:
  @tailrec final def down(n: Int): Int = { if n > 0 then return down(n - 1); 0 }
  def main(args: Array[String]): Unit = println(down(3))
