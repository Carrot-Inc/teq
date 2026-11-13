// A syntax error in another file: scalac runs no phase after its typer once an error is
// reported, so a call of an inline method that is not transparent is never expanded, and
// what its body would report there (an `inline val` that is no constant, an `inline if` that
// cannot reduce) is not reported: the syntax error alone.
// expect: Other.scala:1:14: error: unclosed string literal
// absent: Cannot reduce `inline if`
// absent: inline value must have a literal constant type
// expect: 1 error found
inline def sized(inline n: Int): Int =
  inline val k = n
  inline if k > 5 then 100 else 200
def a = sized((9: Int))
