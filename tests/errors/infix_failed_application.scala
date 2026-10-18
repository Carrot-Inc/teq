// teq: --source future
// An infix operation whose application failed is checked by its tree's symbol, `NoSymbol` (`Typer.typedInfixOp`), and
// the error is hidden where an earlier error covers the operator (`UniqueMessagePositions`): reported after a missing
// operand or argument, hidden by the selection's own error.
class C { def f(x: Int): Int = x }
val c = new C
val a = unknown f 1
val b = c f unknownArg
val d = c nope 1
val e = (c: Any) f 1
// expect: 7:17: error: Alphanumeric method f is not declared infix; it should not be used as infix operator.
// expect: 8:11: error: Alphanumeric method f is not declared infix; it should not be used as infix operator.
// absent: 9:11: error: Alphanumeric
// absent: 10:18: error: Alphanumeric
