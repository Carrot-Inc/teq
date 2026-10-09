// expect: error: expected an expression, found new line
// absent: error: number too large
// A minus before a number is not folded into it across a line break, one inside a block comment
// included: dotc's scanner ends the statement at that break (E018, "expression expected but end of
// statement found"), which scalac places at the break and teq at the token after it: the
// message is pinned alone.
val ok: Float = - /* gap */ 1.5
val x: Int = - /*
*/ 2147483648
