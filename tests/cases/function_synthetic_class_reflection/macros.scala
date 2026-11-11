// The class `newFunctionNType` makes for a synthetic function name (Definitions.scala 129-142): a trait, its type
// parameters `scala$<name>$$T1` .. `scala$<name>$$R` (`paramNamePrefix`), contravariant but the last, covariant; its
// `apply`, whose parameters `syntheticParamNames` names `x$0` on; its parent `Object`. scala-library's `Function1`, which
// teq makes in its place, as the library declares it. The flags and bases teq's reflection gives otherwise for every
// class are pinned apart (tests/pending-cases/repr_symbol_flags_and_bases).
import scala.quoted.*
object M {
  inline def inspect[F]: String = ${ impl[F] }
  def impl[F: Type](using Quotes): Expr[String] = {
    import quotes.reflect.*
    val t = TypeRepr.of[F]
    val s = t.typeSymbol
    def variance(p: Symbol) = if p.flags.is(Flags.Contravariant) then "-" else if p.flags.is(Flags.Covariant) then "+" else ""
    val applies = s.declaredMethods.filter(_.name == "apply").map(_.paramSymss.map(_.map(_.name).mkString("(", ", ", ")")).mkString)
    Expr(s.fullName + ": " + s.declaredTypes.map(p => variance(p) + p.name).mkString(", ") + " | " + applies.mkString("; ") +
      " | AnyRef: " + (t <:< TypeRepr.of[AnyRef]) + " | trait: " + s.flags.is(Flags.Trait))
  }
}
