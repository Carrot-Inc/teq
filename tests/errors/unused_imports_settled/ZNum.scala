object Lib { val n: Int = 1 }
class Num
object Num {
  import Lib.n
  extension (x: Num) def f = n
}

import scala.collection.mutable.Stack

// teq: --werror --wunused imports
// expect: ZNum.scala:8:33: warning: unused import
