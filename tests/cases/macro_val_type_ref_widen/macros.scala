import scala.quoted.*

// A val's `typeRef` is a type reference, which `widen` and `widenTermRefByName` leave as it is;
// its `termRef` widens to the val's declared type.
object M:
  inline def test: String = ${ impl }

  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val s = Symbol.newVal(Symbol.spliceOwner, "x", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val t = s.typeRef
    Expr(s"${t.widen =:= TypeRepr.of[Int]} ${t.widen =:= t} ${s.termRef.widen =:= TypeRepr.of[Int]} " +
      s"${t.widenTermRefByName =:= TypeRepr.of[Int]} ${s.termRef.widenTermRefByName =:= TypeRepr.of[Int]}")
