// An infix call of a right-associative extension method applied to a further clause: the
// application is the function part the clause applies (dotty's `Desugar.binop`, typed with the
// clause's prototype), so the clause's default is the method's, in `rightAssocParams` order: the
// left operand, the receiver, then the default.
def mark(s: String): Int = { print(s); 1 }
extension (r: Int) { def +:(a: Int)(b: Int = mark("B")): Int = a + r + b }
extension (r: Int) { def ++:(a: Int)(b: Int): Int = a + r + b }
@main def run(): Unit =
  println((mark("A") +: mark("R"))())
  println((mark("A") ++: mark("R"))(mark("b")))
  println((mark("A") +: mark("R"))(mark("c")))
