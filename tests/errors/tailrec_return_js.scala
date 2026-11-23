// expect: 8:22: error: Cannot rewrite recursive call: it is not in tail position
// expect: 1 error found
// On JavaScript a def's call of itself under a `return` is written as a call, so `@tailrec` refuses the def
// there, as a call not in tail position (scalac and Scala.js accept it and loop: teq's row). The JVM and the
// interpreter accept it and loop (tests/cases/return_tail_call_tailrec).
import scala.annotation.tailrec
object Down:
  @tailrec final def down(n: Int): Int = { if n > 0 then return down(n - 1); 0 }
