// expect: 16:18: error: inline value must be pure but was: yes()
// expect: 18:34: error: inline value must be pure but was: yes()
// expect: inlined from tests/errors/inline_val_purity.scala:13
// expect: 2 errors found
// An `inline val` whose constant is its initialiser's type is that constant where the
// initialiser is pure (scalac 3.8.4's `exprPurity` in `InlineVals`): a call of `def yes(): true`
// is not, effects or none, at the val of a plain method and where an inline body expands, the
// same two errors as scalac's.
def yes(): true =
  println("effect")
  true
inline def f(): Boolean =
  inline val x: Boolean = yes()
  x
def g(): Boolean =
  inline val y = yes()
  y
@main def main(): Unit = println(f() && g())
