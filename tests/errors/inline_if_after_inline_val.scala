// An `inline if` an expansion cannot reduce, after an `inline val` of the same body whose value
// is no constant: scalac reports the `inline if` alone, at the call; its check of the inline
// value is a later phase's, which does not run once the typer reported an error. The inline val
// alone is tests/errors/inline_val_ascribed_body.scala.
// expect: 12:11: error: Cannot reduce `inline if` because its condition is not a constant value: k > 5
// absent: 12:11: error: inline value must have a literal constant type
// expect: 1 error found
transparent inline def sized(inline n: Int): Int =
  inline val k = n
  inline if k > 5 then 100 else 200
@main def run(): Unit =
  println(sized((9: Int)))
