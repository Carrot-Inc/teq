// An inline argument a macro sees is the tree the call site typed, its subtrees as written (`tryInlineArg` gives the
// binding's right-hand side, not retyped): a constant operation inside it stays an operation; only an argument that
// is itself a constant path stands for its literal (`computeParamBindings`'s singleton binding). An operation written
// in an inline body is typed again where the body expands (InlineTyper's `constToLiteral`): `wrap`'s `1 + 2` is `3`.
import scala.quoted.*
object Mac:
  inline def describe(inline x: Any): String = ${ describeImpl('x) }
  def describeImpl(x: Expr[Any])(using Quotes): Expr[String] =
    x match
      case '{ ($a: Int) + ($b: Int) } => '{ "sum(" + ${ describeImpl(a) } + "," + ${ describeImpl(b) } + ")" }
      case '{ $n: Int } => n match
        case '{ 3 } => Expr("three")
        case _ => Expr("int")
      case _ => Expr("else: " + x.show)
  inline def isLit(inline x: Any): String = ${ isLitImpl('x) }
  def isLitImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def strip(t: Term): Term = t match
      case Inlined(_, Nil, e) => strip(e)
      case _ => t
    def kind(t: Term): String = strip(t) match
      case Literal(_) => "lit"
      case _ => "tree"
    Expr(strip(x.asTerm) match
      case Literal(_) => "literal"
      case Apply(_, args) => "apply(" + args.map(kind).mkString(",") + ")"
      case Lambda(_, body) => "lambda(" + (strip(body) match { case Apply(_, List(a)) => kind(a); case _ => "?" }) + ")"
      case _ => "other")
object W:
  inline def wrap(inline x: Int): String = Mac.describe((1 + 2) + x)
