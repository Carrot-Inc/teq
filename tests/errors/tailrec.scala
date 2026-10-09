// expect: 8:24: error: TailRec optimisation not applicable, method f is neither private nor final so can be overridden
// expect: 11:16: error: Cannot rewrite recursive call: it is not in tail position
// expect: 12:16: error: TailRec optimisation not applicable, method h contains no recursive calls
// expect: 13:24: error: Cannot rewrite recursive call: it is not in tail position
// expect: 16:16: error: Cannot rewrite recursive call: it is not in tail position
// expect: 5 errors found
import scala.annotation.tailrec
class C { @tailrec def f(n: Int): Int = if n == 0 then 0 else f(n - 1) }
object O:
  @tailrec final def ok(n: Int): Int = if n == 0 then 0 else ok(n - 1)
  @tailrec def g(n: Int): Int = if n == 0 then 0 else 1 + g(n - 1)
  @tailrec def h(n: Int): Int = n
  @tailrec private def quux(xs: List[String]): List[String] = quux(quux(xs))
trait T { @tailrec final def k(n: Int): Int = if n == 0 then 0 else k(n - 1) }
object B:
  @tailrec def strict(n: Int): Boolean = n <= 0 | strict(n - 1)
  @tailrec def short(n: Int): Boolean = n <= 0 || short(n - 1)
