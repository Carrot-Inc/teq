// `@tailrec` over a plain inline call whose inline arguments hold the recursive calls: the check
// counts the calls the expanded body reaches, as scalac's `TailRec` walks the tree after `Inlining`,
// not the argument trees the expansion copied into its branches.
import scala.annotation.tailrec
inline def choose(c: Boolean, inline a: Int, inline b: Int): Int = if c then a else b
object O:
  @tailrec def loop(n: Int): Int = choose(n == 0, 7, loop(n - 1))
  @tailrec def both(n: Int, acc: Int): Int = choose(n <= 0, acc, choose(n % 2 == 0, both(n - 2, acc + 2), both(n - 1, acc + 1)))
@main def run(): Unit =
  println(O.loop(3))
  println(O.both(100001, 0))
