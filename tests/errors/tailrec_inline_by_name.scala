// jars: scala-library
// teq: --target jvm --std=scala-library
// expect: 8:22: error: Cannot rewrite recursive call: it is not in tail position
// expect: 1 error found
import scala.annotation.tailrec
object TR8:
  inline def twice(x: => Int): Int = x + x
  @tailrec final def viaInlineByName(n: Int): Int = twice { if n > 0 then return viaInlineByName(n - 1); 1 }
