// A named argument's tree spans its `name = expr` (the parser's `cpy.NamedArg` of the `Assign`, which
// `typedNamedArg`'s `cpy.NamedArg(tree)` keeps on the typed tree), and reflection's `pos.sourceCode`
// reads the source at that span (`PositionMethods.sourceCode`): in order and reordered (`liftArg`'s copy
// around the lifted value), past a default, to a constructor, `copy`, an overloaded method (typed
// ahead, `FunProto.typedArgs`) and a function literal, and in an inline method's body, of this file and
// another. scalac prints the lines of the .expected file.
import scala.quoted.*
object Show:
  inline def args(inline e: Any): String = ${ argsImpl('e) }
  def argsImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def src(t: Tree): String = t.pos.sourceCode.getOrElse("<none>")
    def at(t: Tree): String = s"${t.pos.startLine}:${t.pos.startColumn}-${t.pos.endLine}:${t.pos.endColumn}"
    val out = List.newBuilder[String]
    new TreeTraverser:
      override def traverseTree(t: Tree)(owner: Symbol): Unit =
        t match
          case n @ NamedArg(name, v) => out += s"NamedArg $name [${src(n)}] ${at(n)} value [${src(v)}] ${at(v)}"
          case _ =>
        super.traverseTree(t)(owner)
    .traverseTree(e.asTerm)(Symbol.spliceOwner)
    Expr(out.result().mkString("\n"))
