// expect: 9:8: error: cannot resolve import: Nope not found
// expect: 27:5: error: value += is not a member of Int
// expect: 27:10: error: Not found: missing
// expect: 3 errors found
// An operand of an assignment operator that is an error without a report of its own (a name of an import that did
// not resolve: told at the import) fails neither typing of the operation: the assignment form stands and nothing
// more is told, on a var, a field, an element and a String. An operand whose own error is told fails both, and the
// first failure, the operator's, is told beside it (scalac's two lines).
import Nope.t
class Box(var v: Int)
class Acc(var n: Int):
  def +=(k: Int): Unit = n += k
  def +(k: Int): Acc = new Acc(n + k)
object A:
  val bx = new Box(5)
  def getBox(): Box = bx
  def f(): Int =
    var x = 1
    x += t("r", 1)
    getBox().v += t("rhs", 1)
    val arr = new Array[Int](2)
    arr(0) += t("e", 2)
    var acc = new Acc(1)
    acc += t("m", 3)
    var s = "a"
    s += t("s", 4)
    x += missing.field
    x
