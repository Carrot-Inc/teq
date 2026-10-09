// An `inline val` in an inline body whose initialiser is an ascription of the inline parameter
// (`(i: Int)`), or a plain inline method's call (scala3's i24420 `f3`, which scalac 3.8.4
// rejects), is no constant where the body expands, as scalac checks it after inlining: the
// error is at the expansion. An operation over the call folds there (`f1() + 1L`,
// tests/cases/inline_definition_inline_val_expr.scala).
// expect: 16:11: error: inline value must have a literal constant type
// expect: 17:11: error: inline value must have a literal constant type
// expect: 18:11: error: inline value must have a literal constant type
transparent inline def body(inline i: Int) = { inline val j = (i: Int); j }
inline def f1(): Long = 1L
inline def f3(): Long =
  inline val x = f1()
  x
inline def plain: Int = { inline val k = (f1().toInt: Int); k }
@main def run(): Unit =
  println(body(0))
  println(f3())
  println(plain)
