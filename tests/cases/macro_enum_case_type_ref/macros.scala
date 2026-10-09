import scala.quoted.*

// A sealed enum's children as chimney's `SealedHierarchies` reads them: a singleton case's
// `typeRef` is a type whose `typeSymbol` is the case's val and which has no `termSymbol`, while
// the singleton type written in source has the enum's class; `Ref` of the val is the case.
object Probe:
  inline def children[A]: String = ${ childrenImpl[A] }
  def childrenImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    def show(s: Symbol): String = s"${s.name}(term=${s.isTerm}, case=${s.flags.is(Flags.Case)}, enum=${s.flags.is(Flags.Enum)})"
    val kids = TypeRepr.of[A].typeSymbol.children.map { c =>
      val t = c.typeRef
      val noTerm = if c.isTerm then s" noTermSymbol=${t.termSymbol.isNoSymbol}" else ""
      s"${show(c)} typeSymbol=${show(t.typeSymbol)}$noTerm sub=${t <:< TypeRepr.of[A]}"
    }
    Expr(kids.mkString("\n"))

  inline def singletonSymbol[A]: String = ${ singletonSymbolImpl[A] }
  def singletonSymbolImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    Expr(s"${TypeRepr.of[A].typeSymbol.name} ${TypeRepr.of[A].termSymbol.name}")

  inline def first[A]: A = ${ firstImpl[A] }
  def firstImpl[A: Type](using q: Quotes): Expr[A] =
    import q.reflect.*
    val child = TypeRepr.of[A].typeSymbol.children.head
    val tpe = child.typeRef
    val sym = if tpe.termSymbol != Symbol.noSymbol then tpe.termSymbol else tpe.typeSymbol
    Ref(sym).asExprOf[A]
