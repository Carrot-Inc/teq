// A constructor of several parameter lists is applied one list at a time, as a method is
// (`Applications`' application of each clause in turn), so a macro sees `new C(a)(b)` as
// `Apply(Apply(Select(New(C), <init>), a), b)`, one `Apply` per clause (`Trees.Apply`): a type
// argument's `TypeApply` innermost (the constructor's `PolyType` instantiated, `Typer.adapt`), a
// `using` clause in a list of its own, a by-name argument the expression; the inner applications are
// of the remaining clauses' method type and name the constructor. scalac prints the lines of the
// .expected file.
import scala.quoted.*
object Shape:
  inline def of(inline e: Any): String = ${ ofImpl('e) }
  def ofImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def shape(t: Tree): String = t match
      case Inlined(_, Nil, b) => shape(b)
      case Apply(f, as) => s"Apply(${shape(f)}, [${as.map(shape).mkString(", ")}])"
      case TypeApply(f, ts) => s"TypeApply(${shape(f)}, [${ts.map(_.tpe.typeSymbol.name).mkString(", ")}])"
      case Select(q, n) => s"Select(${shape(q)}, $n)"
      case New(tpt) => s"New(${tpt.tpe.typeSymbol.name})"
      case NamedArg(n, v) => s"NamedArg($n, ${shape(v)})"
      case Literal(c) => s"Literal(${c.value})"
      case Ident(n) => s"Ident($n)"
      case other => s"<${other.getClass.getSimpleName}>"
    Expr(shape(e.asTerm))
  inline def inner(inline e: Any): String = ${ innerImpl('e) }
  def innerImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def strip(t: Term): Term = t match
      case Inlined(_, Nil, b) => strip(b)
      case _ => t
    val Apply(f, _) = strip(e.asTerm): @unchecked
    Expr(s"${f.tpe.widen.show} ${f.symbol.name} ${f.symbol.owner.name}")
