// An `inline val` whose constant is its initialiser's type stands for it where the initialiser
// is pure, as scalac 3.8.4's `exprPurity` takes it: a stable path (`C.flag` of a `final val`)
// and an inline call that expands to its literal.
object C:
  final val flag = true
inline def one(): 1 = 1
inline def f(): Boolean =
  inline val x: Boolean = C.flag
  x
inline def g(): Int =
  inline val x: Int = one()
  x
@main def main(): Unit =
  println(f())
  println(g())
