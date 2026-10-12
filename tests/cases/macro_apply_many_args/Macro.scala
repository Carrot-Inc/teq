// 3,000 applications of a method of twenty parameters in one clause, the shape whose cost the
// application's arity check adds to (its clauses read where they are kept, not copied).
import scala.quoted.*
object M:
  def f(a0: Int, a1: Int, a2: Int, a3: Int, a4: Int, a5: Int, a6: Int, a7: Int, a8: Int, a9: Int, a10: Int, a11: Int, a12: Int, a13: Int, a14: Int, a15: Int, a16: Int, a17: Int, a18: Int, a19: Int): Int = a0
  inline def test: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val fn = Select.unique('{ M }.asTerm, "f")
    val args = List.fill(20)(Literal(IntConstant(1)))
    var i = 0
    while i < 3000 do
      val call = Apply(fn, args)
      i += 1
    Expr(i)
