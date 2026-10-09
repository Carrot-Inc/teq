import scala.quoted.*

// Operators of the builtin types rebuilt by name through `quotes.reflect`, as zio-test's
// `assertTrue` and munit's assertions do: `!b` and `n * 2` have no member symbol in teq.
object Macros:
  inline def negated(inline b: Boolean): Boolean = ${ negatedImpl('b) }
  def negatedImpl(b: Expr[Boolean])(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    Select.unique(b.asTerm, "unary_!").asExprOf[Boolean]

  inline def doubled(inline n: Int): Int = ${ doubledImpl('n) }
  def doubledImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    Select.overloaded(n.asTerm, "*", Nil, List(Literal(IntConstant(2)))).asExprOf[Int]

  inline def negative(inline n: Int): Int = ${ negativeImpl('n) }
  def negativeImpl(n: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    Select.unique(n.asTerm, "unary_-").asExprOf[Int]

  inline def joined(inline s: String, inline t: String): String = ${ joinedImpl('s, 't) }
  def joinedImpl(s: Expr[String], t: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Select.overloaded(s.asTerm, "+", Nil, List(t.asTerm)).asExprOf[String]
