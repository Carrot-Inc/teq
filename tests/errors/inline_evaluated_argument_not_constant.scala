// An argument of an inline call that is evaluated at compile time is no constant of the language: an `inline if`
// over the parameter is refused, while an inline match over one falls to its default case, as scalac 3.8.4 has it
// (L:C is where its caret begins, 1-based).
// expect: 12:11: error: Cannot reduce `inline if` because its condition is not a constant value:
// expect: 1 error found
inline def pick(inline flag: Boolean): String = inline if flag then "yes" else "no"
inline def kind(inline n: Int): String = inline n match
  case 3 => "three"
  case _ => "other"

@main def run(): Unit =
  println(pick("abc".startsWith("a")))
  println(kind("ab".length + 1))
