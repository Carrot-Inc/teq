// jars: scala-library
// teq: --target jvm --std=scala-library
// expect: 7:22: error: Cannot rewrite recursive call: it is not in tail position
// expect: 1 error found
import scala.annotation.tailrec
object TR7:
  @tailrec final def lz(n: Int): Int = { lazy val x: Int = { if n > 0 then return lz(n - 1); 5 }; x + 1 }
