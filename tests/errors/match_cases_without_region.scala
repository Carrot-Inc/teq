// expect: 5:19: error: expected '{', found 'case'
// The cases of a `match` stand in braces or in an indented block of their own: a `case` on the `match`'s own
// line, with neither, is refused. scalac: "'{' expected, but 'case' found" at the `case`.
def f(x: Int): String =
  val s = x match case 1 => "a"
  s
