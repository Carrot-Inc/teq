// expect: 18:35: error: value += is not a member of Int
// expect: 18:64: error: type mismatch: found "s", required Int
// expect: 19:68: error: type mismatch: found "s", required Int
// expect: 20:42: error: value += is not a member of Int
// expect: 20:76: error: type mismatch: found "s", required Int
// expect: 21:32: error: value += is not a member of Int
// expect: 21:63: error: type mismatch: found "s", required Int
// expect: 22:41: error: value -= is not a member of String
// expect: 22:70: error: type mismatch: found "s", required Int
// expect: 9 errors found
// The body of an anonymous class in an operand of an assignment operator is typed with each typing of the
// operand: what a dropped typing told of it goes with that typing, and the typing that is kept tells it. Where
// the operator is no member and the assignment form fails on the body, the first failure stands with the body's
// own line; in an index the body's line alone. scalac tells these nine lines.
class Pub(val a: Int = 1)
class Cell { var v: Int = 0 }
object Test:
  def onVar(): Int = { var n = 0; n += (new Pub { def y: Int = "s" }).a; n }
  def inIndex(arr: Array[Int]): Unit = arr((new Pub { def y: Int = "s" }).a) += 1
  def onElement(arr: Array[Int]): Unit = arr(0) += (new Pub { def y: Int = "s" }).a
  def onField(c: Cell): Unit = c.v += (new Pub { def y: Int = "s" }).a
  def bothFail(): Unit = { var s = "a"; s -= (new Pub { def y: Int = "s" }).a }
