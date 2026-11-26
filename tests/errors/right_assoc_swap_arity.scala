// A right-associative extension's own first clause must have a single parameter.
extension (x: Int)
  def %:(a: String, b: String): String = a + b
@main def run(): Unit = println("ok")
// expect: 3:7: error: right-associative extension method must start with a single parameter, consider a tupled parameter instead
