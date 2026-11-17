// expect: 11:12: error: index out of bounds: 5
// expect: 13:17: error: index out of bounds: 6
// expect: 2 errors found
// An operand of an assignment operator whose record is a later phase's (a tuple's element out of its bounds: told
// where the reference expands the selection) fails neither typing of the operation: the assignment form stands and
// the index alone is told, on a var and on an element.
object A:
  def h(): Int =
    var x = 1
    val t = (1, "s")
    x += t(5).hashCode
    val arr = new Array[Int](2)
    arr(0) += t(6).hashCode
    x
