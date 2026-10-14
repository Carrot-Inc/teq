// expect: 7:21: error: reassignment to val text
// A placeholder alone as a named argument's value makes the argument a function of an assignment,
// `x => text = x`, as dotty parses an argument as an expression (`Parsers.expr`, `isWildcard`
// through the ascription and the parentheses): `copy` gets the function, whose body assigns the
// case class's val.
case class State(text: String):
  val update = copy(text = (_: String))
