object Lib { val n: Int = 1 }
class Num
object Num {
  import Lib.n
  extension (x: Num) def f = n
}
