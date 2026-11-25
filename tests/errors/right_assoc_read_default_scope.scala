// A right-associative extension's right operand's default cannot name the receiver: the clauses are swapped
// before the defaults are made, so the receiver is not in scope there.
extension (l: Int)
  def +:(r: Int = l): Int = l + r
@main def run(): Unit = println(+:()(1))
// expect: 4:19: error: Not found: l
