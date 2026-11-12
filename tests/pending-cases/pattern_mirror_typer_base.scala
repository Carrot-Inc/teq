case class P(x: Int)
object Use:
  def f(m: scala.deriving.Mirror.Product): Int = m match
    case p: P.type => p

// expect: 4:23: error: Found:    (p : P.type & scala.deriving.Mirror.Product)
// expect: Required: Int
// expect: 1 error found

// Pending on the TypeMismatch port
// (messages.scala:311-371), the diagnostic's text.
// The expected messages are scalac's.
