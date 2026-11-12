// A member without a body is `Deferred`, a using clause's parameter `Given` and its method type `isImplicit`
// (`Flags.Deferred`, `Flags.Given`, `MethodType.isImplicit` of a `ContextualMethodType`), a written trait's and the
// `apply` of a context function class (`Definitions.newFunctionNType`, `MethodType.companion(isContextual)`) alike.
// teq's reflection gives none of the three. Pinned with scalac's answer.
import scala.quoted.*
object M {
  inline def inspect[F]: String = ${ impl[F] }
  def impl[F: Type](using Quotes): Expr[String] = {
    import quotes.reflect.*
    val t = TypeRepr.of[F]
    val s = t.typeSymbol
    val methods = s.declaredMethods.sortBy(_.name).map { m =>
      val implicitType = t.memberType(m) match
        case mt: MethodType => mt.isImplicit
        case _ => false
      m.name + m.paramSymss.map(ps => ps.map(p => (if p.flags.is(Flags.Given) then "given " else "") + p.name).mkString("(", ", ", ")")).mkString +
        (if m.flags.is(Flags.Deferred) then " deferred" else "") + (if implicitType then " implicit" else "")
    }
    Expr(s.fullName + ": " + methods.mkString("; "))
  }
}
