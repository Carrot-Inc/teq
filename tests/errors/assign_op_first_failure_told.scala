// expect: 23:5: error: value += is not a member of Int
// expect: 25:5: error: value += is not a member of Int
// expect: 27:5: error: value += is not a member of Char
// expect: 29:5: error: value += is not a member of Int
// expect: 29:15: error: not found: nope
// expect: 30:5: error: value += is not a member of Int
// expect: 32:5: error: value ++= is not a member of List[Int]
// expect: 32:35: error: value nope is not a member of Int
// expect: 34:5: error: value += is not a member of Int
// expect: 36:5: error: value -= is not a member of String
// expect: 37:5: error: value += is not a member of List[Int]
// expect: 37:11: error: not found: nope
// expect: 12 errors found
// Where both tries of an assignment operator's application fail, the first failure stands (`typedApply`'s
// `failedState.commit()`): the operator is no member of the left side's type, with what typing the right side
// told; never the assignment form's own words (`value update is not a member`, `value - is not a member`, a
// reassignment). scalac tells an error at each of these lines, two at the ones whose right side has its own.
class Box(val n: Int):
  def apply(i: Int): Int = n + i
object Test:
  def main(args: Array[String]): Unit =
    val b = new Box(1)
    b(1) += 2
    val xs = List(1)
    xs(0) += 1
    val text = "abc"
    text(0) += 1
    val arr = Array(1, 2)
    arr(0) += nope
    arr(0) += "s"
    val lists = Array(List(1))
    lists(0) ++= List(1).map(x => x.nope)
    val v = 1
    v += 2
    var s = "a"
    s -= 1
    xs += nope
