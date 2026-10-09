// An `inline val` whose initialiser is ascribed a type that is no literal type (`(0: Int)`) is
// no constant, as scalac types the ascription; nor is one of a plain inline method's call, which
// scalac's typer keeps as a call at its declared type, an operation over one, or an ascription
// of either: "inline value must have a literal constant type" at the definition. In an inline
// body such a value is checked at the expansion (scalac's check runs after the inlining phase,
// tests/errors/inline_val_ascribed_body.scala). A transparent method's constant is one.
// expect: 15:19: error: inline value must have a literal constant type
// expect: 16:22: error: inline value must have a literal constant type
// expect: 17:19: error: inline value must have a literal constant type
// expect: 18:28: error: inline value must have a literal constant type
// expect: 19:27: error: inline value must have a literal constant type
inline def zero: Int = 0
transparent inline def tzero = 0
object Consts:
  inline val k = (0: Int)
  inline val plain = zero
  inline val op = zero + 0
  inline val ascribedOp = ((zero + 0): Int)
  inline val opAscribed = (zero: Int) + 0
  inline val t = tzero
transparent inline def body(inline i: Int) = { inline val j = (i: Int); j }
@main def run(): Unit =
  println(Consts.k)
  println(Consts.t)
