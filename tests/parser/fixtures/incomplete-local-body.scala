// An incomplete local val that keeps a body after its skip to `=` has the error type, as a
// member has: its use is no mismatch, and its body is typed for its own errors.
def demo(): Unit =
  val x foo = "s"
  val y: Int = x
  val z foo = 1 + true
