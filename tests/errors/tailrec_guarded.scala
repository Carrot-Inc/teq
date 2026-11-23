// jars: scala-library
// teq: --target jvm --std=scala-library
// expect: 11:22: error: Cannot rewrite recursive call: it is not in tail position
// expect: 12:22: error: Cannot rewrite recursive call: it is not in tail position
// expect: 2 errors found
// A call under a `return` that a finalizer or a guarded handler guards is no loop's jump on the JVM: scalac 3.8.4
// rewrites both and runs them wrongly, teq refuses them; one nothing guards is a jump in both (on JavaScript teq
// refuses that one too: tailrec_return_js).
import scala.annotation.tailrec
object Guarded:
  @tailrec final def throughFinally(n: Int): Int = try { if n > 0 then return throughFinally(n - 1); 0 } finally println("f")
  @tailrec final def throughCatch(n: Int): Int =
    try { if n > 0 then return throughCatch(n - 1); throw new IllegalStateException("e") }
    catch { case _: IllegalStateException if n == 5 => 42 }
  @tailrec final def free(n: Int): Int = { if n > 0 then return free(n - 1); 0 }
