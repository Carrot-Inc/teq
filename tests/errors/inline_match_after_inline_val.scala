// An `inline match` an expansion cannot reduce, after an `inline val` of the same body whose
// value is no constant: scalac reports the match alone, at the call; its check of the inline
// value is a later phase's, which does not run once the typer reported an error
// (tests/errors/inline_if_after_inline_val.scala is the same with an `inline if`).
// expect: 13:11: error: cannot reduce inline match with
// absent: 13:11: error: inline value must have a literal constant type
// expect: 1 error found
transparent inline def named(inline n: Int): String =
  inline val k = n
  inline k match
    case 9 => "nine"
@main def run(): Unit =
  println(named((9: Int)))
