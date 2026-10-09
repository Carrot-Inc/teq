package erased

import scala.annotation.targetName

// Two transparent overloads whose parameters erase alike and whose results differ, as
// `@targetName` lets them: the downstream's reader tells the recorded callee by its whole
// signature, the result's erasure included.
object A:
  transparent inline def first(xs: List[Int]): Int = xs.head
  @targetName("firstString")
  transparent inline def first(xs: List[String]): String = xs.head
