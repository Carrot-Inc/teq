// A right-associative extension's right parameter cannot depend on the receiver, swapped after it.
extension (l: Int)
  def +:(r: l.type): Int = l
@main def run(): Unit = ()
// expect: 3:13: error: Not found: l
