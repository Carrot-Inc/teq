case class P(x: Int)
object Use:
  def f(m: scala.deriving.Mirror.Product): Int = m match
    case p: P.type => p

// expect: 4:23: error: Found:    (p : P.type & scala.deriving.Mirror.Product)
// expect: Required: Int
// expect: 1 error found

// Remaining diagnostic text: C1 TypeMismatch port (messages.scala:311-371),
// in C1's set-aside batch. The typedBind semantic unit checks the actual
// intersection and rejection as Int. Moved by session b's 2026-11-06 ruling.
