// A right-associative extension's receiver cannot name a parameter of the method's own clauses.
extension (l: r.type)
  def *:(r: Int): Int = r
@main def run(): Unit = ()
// expect: 2:15: error: right-associative extension method cannot have a forward reference to r
