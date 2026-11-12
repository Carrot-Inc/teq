val f1 = unknown1 _
val f2 = unknown2 _
// HideNonSensicalMessages.isHidden (HideNonSensicalMessages.scala:14-19): once an error was reported, a message whose
// printer met an erroneous type (Message.Printer.toText, Message.scala:305-307) is not shown. scalac 3.8.4 gives two
// errors: line 1's E099, its operand's type erroneous but no error reported before it; line 2's E006 alone, its E099
// hidden.
// expect: error: Only function types can be followed by _ but the current expression has type <error
// absent: 2:10: error: Only function types can be followed by _
// Where the expectation stops short of scalac's capture, each with its owner (so no count line yet):
// - E099's place: scalac's 1:18 (the `_`): Typer's typedAsFunction.
// - `<error Not found: unknown1>`: PlainPrinter.toText prints an ErrorType's message
//   (typedIdent's errorType of NotFoundMsg).
// - Line 1's `Not found: unknown1`, which scalac's UniqueMessagePositions hides behind the E099 reported before it (the
//   operand's nested state is committed after the report): the reporter's UniqueMessagePositions.
// - E006's text, scalac's `Not found: unknown2` (messages.scala's NotFoundMsg).
