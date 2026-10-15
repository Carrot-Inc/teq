package ident
import scala.quoted.*
// The module class of a file's `$package` object (`Desugar.packageDef`'s, the namer's here) as reflection
// reads it: the class whose `typeRef` is a `TypeRef` and whose module's `termRef` a `TermRef`, the owner of the
// file's opaque type and of its extension, and a written object's qualifier the package.
object Inspect:
  inline def test: String = ${impl}
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val owner = TypeRepr.of[Token] match
      case TypeRef(q, _) => q.termSymbol.moduleClass
      case _ => Symbol.noSymbol
    def kind(t: TypeRepr): String = t match
      case _: TypeRef => "type"
      case _: TermRef => "term"
      case _ => "other"
    def called(t: Term): Symbol = t match
      case Inlined(_, _, body) => called(body)
      case Apply(fn, _) => fn.symbol
      case t => t.symbol
    val extensionOwner = called('{Token(1).twice}.asTerm).owner
    val q = TypeRepr.of[Top.type] match
      case TermRef(q, _) => q.termSymbol.moduleClass
      case _ => Symbol.noSymbol
    Expr(kind(owner.typeRef) + ":" + kind(owner.companionModule.termRef) + ":" + owner.name + ":" + (owner == extensionOwner) + ":" + (owner == TypeRepr.of[Token].typeSymbol.owner) + ":" + q.name)
