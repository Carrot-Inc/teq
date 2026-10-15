// A blank line after an expression's trailing operator, outside a case region: refused.
// expect: 4:14: error: expected an expression, found a blank line
@main def run(): Unit =
  val x = 1 +

    2
  println(x)
