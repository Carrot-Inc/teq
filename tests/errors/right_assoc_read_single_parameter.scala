// A right-associative extension's first clause must be a single parameter.
extension (l: Int)
  def /:(r: Int, s: Int): Int = l
@main def run(): Unit = ()
// expect: 3:7: error: right-associative extension method must start with a single parameter, consider a tupled parameter instead
