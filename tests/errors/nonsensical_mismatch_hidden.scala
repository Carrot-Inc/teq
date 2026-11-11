type M[X] <: Int = X match
  case Int => List[Undefined]
// A match type case's body checked against the bound (typedType(body, pt)): the TypeMismatch of `List[<error>]` prints
// an erroneous type, and an error was reported before it, so HideNonSensicalMessages hides it
// (HideNonSensicalMessages.scala:14-19; Message.Printer.toText, Message.scala:305-307). scalac 3.8.4 gives the E006
// alone.
// absent: 2:15: error
// expect: 1 error found
// teq's E006 at 2:20 still reads `type Undefined not found` for scalac's `Not found: type Undefined` (messages.scala's
// NotFoundMsg).
