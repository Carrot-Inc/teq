// A failed selection is an error tree, with no resolved term symbol.
// Binding operands reads TreeInfo.exprPurity, as LiftImpure does.
// expect: 5:20: error: not found: absent
object Main:
  val value = 1 :: absent.values
// expect: 1 error found
