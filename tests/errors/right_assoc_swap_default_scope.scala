// The default of a right-associative extension's own parameter cannot read the receiver, which its clauses follow.
object Lib:
  extension (x: String)
    def %:(y: String = x): String = x + y
@main def run(): Unit = println("ok")
// expect: 4:24: error: Not found: x
