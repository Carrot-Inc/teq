import scala.quoted.*

// A val's `typeRef` keeps the val as its type symbol after the val's `termRef` is asked for,
// and asking again gives an equal reference.
object M:
  inline def test: String = ${ impl }

  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val s = Symbol.newVal(Symbol.spliceOwner, "x", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val a = s.typeRef
    val before = a.typeSymbol == s
    val b = s.termRef
    val after = a.typeSymbol == s
    Expr(s"$before $after ${b.termSymbol == s} ${a =:= s.typeRef} ${b.typeSymbol == s}")
