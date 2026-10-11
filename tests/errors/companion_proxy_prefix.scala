// expect: 12:12: error: Test is not a valid class prefix, since it is not an immutable path
// expect: 1 error found

// The constructor proxy a companion without an `apply` gets constructs through the companion's
// prefix, an immutable path (dotty's `newExpr`, E083); a stable value is one.
class Test:
  class A:
    def run = "A"
  object A
  def ok(t: Test): String =
    t.A().run
  val x1 = new Test().A()
