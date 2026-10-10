// expect: 8:11: error: O is not a valid class prefix, since it is not an immutable path
// expect: 1 error found

// A creator application's prefix is an immutable path, as a `new`'s (dotty's `newExpr`, E083).
class O(val n: Int):
  class Inner(val x: Int)
@main def run(): Unit =
  println(new O(9).Inner(1).x)
