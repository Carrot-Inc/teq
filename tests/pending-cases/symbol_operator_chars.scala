// Operators of non-ASCII math and other symbols (Unicode Sm, So: `∑`, `©=`), which scalac 3.8.4's scanner reads
// as operator characters (`isOperatorPart`) and teq's lexer as letters ("5 errors found"); `©=` an assignment's
// precedence, so `2 ©= 3 + 1` is `2 ©= (3 + 1)`.
object O {
  extension (x: Int) def ∑(y: Int): Int = x + y
  extension (x: Int) def ©=(y: Int): Int = x * y
  val a = 2 ∑ 3
  val b = 2 ©= 3 + 1
}
@main def run(): Unit = println((O.a, O.b))
