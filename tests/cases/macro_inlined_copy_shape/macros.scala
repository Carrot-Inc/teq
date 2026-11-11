import scala.quoted.*

object Shape:
  inline def inspect(inline x: Int): String = ${ inspectImpl('x) }
  inline def mapped(inline x: Int): Int = ${ mappedImpl('x) }
  inline def emptyBlock(inline x: Int): Int = ${ emptyBlockImpl('x) }

  def inspectImpl(x: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def shape(t: Term): String = t match
      case Inlined(None, Nil, e) => shape(e)
      case Inlined(Some(_), _, _) => "inline"
      case Block(stats, e) => "block" + stats.size + "[" + shape(e) + "]"
      case Apply(Select(_, name), args) => name + args.map(shape).mkString("[", ",", "]")
      case _ => t.show(using Printer.TreeStructure)
    Expr(shape(x.asTerm))

  def mappedImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val copier = new TreeMap:
      override def transformTerm(t: Term)(owner: Symbol): Term = t match
        case i @ Inlined(call, bindings, expansion) =>
          Inlined.copy(i)(call, bindings, super.transformTerm(expansion)(owner))
        case _ => super.transformTerm(t)(owner)
    copier.transformTerm(x.asTerm)(Symbol.spliceOwner).asExprOf[Int]

  def emptyBlockImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    Block(Nil, x.asTerm).asExprOf[Int]

  inline def blocks(inline x: Int): Int = ${ blocksImpl('x) }
  def blocksImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    def count(t: Term): Int = t match
      case Inlined(_, _, e) => count(e)
      case Block(stats, _) => stats.size
      case _ => -1
    Expr(count(x.asTerm))
