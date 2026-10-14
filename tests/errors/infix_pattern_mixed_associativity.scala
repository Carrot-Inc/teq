// Two infix patterns of one precedence and different associativity may not be mixed: dotty's `infixPattern`
// reduces its operators as a term's do (`reduceStack`), and `checkAssoc` refuses the pair at the operand after
// the stacked operator.
object + { def unapply(x: Int): Option[(Int, Int)] = Some((x, x)) }
object +: { def unapply(x: Int): Option[(Int, Int)] = Some((x, x)) }
def test(x: Int): Int = x match
  case a + b +: c => a
  case a +: b +: c => b
  case a + b + c => c
// expect: 7:12: error: + (which is left-associative) and +: (which is right-associative) have same precedence and may not be mixed
