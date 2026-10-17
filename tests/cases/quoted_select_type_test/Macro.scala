// jars: scala-library cats-kernel cats-core
//> using dep org.typelevel::cats-core:2.13.0
// A tree that `quotes.reflect`'s type test takes for a `Select` is one `Select.unapply` takes apart
// (`QuotesImpl`'s `SelectTypeTest` and `SelectModule.unapply`), over the shapes zio-test's smart
// assertion reads: an extension through a conversion (cats' `===`), a symbolic instance call, an
// instance member and an object's own member.
import scala.quoted.*
object Inspect:
  inline def inspect(inline e: Any): String = ${ impl('e) }
  def impl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    var tested = 0
    var extracted = 0
    val walk = new TreeTraverser:
      override def traverseTree(t: Tree)(owner: Symbol): Unit =
        t match
          case s: Select =>
            tested += 1
            Select.unapply(s) match
              case (_, name) if name.nonEmpty => extracted += 1
              case _ => ()
          case _ => ()
        super.traverseTree(t)(owner)
    walk.traverseTree(e.asTerm)(Symbol.spliceOwner)
    def strip(t: Term): Term = t match
      case Inlined(_, _, b) => strip(b)
      case Typed(b, _) => strip(b)
      case Block(Nil, b) => strip(b)
      case _ => t
    def kind(t: Term): String = t match
      case Apply(f, args) => s"Apply(${f.symbol.name}, ${args.size})"
      case Select(_, n) => s"Select($n)"
      case Ident(n) => s"Ident($n)"
      case This(_) => "This"
      case _ => "other"
    val top = strip(e.asTerm) match
      case Apply(Select(q, n), args) => s"Apply(Select(${kind(strip(q))}, $n), ${args.size})"
      case other => kind(other)
    Expr(s"$top ${if tested == extracted then "every Select extracted" else s"$tested tested, $extracted extracted"}")
