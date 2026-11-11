import scala.quoted.*
object Mac:
  inline def sumArgs(inline x: Int): Int = ${ sumImpl('x) }
  def sumImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    def strip(t: Term): Term = t match
      case Inlined(_, Nil, e) => strip(e)
      case Block(Nil, e) => strip(e)
      case _ => t
    strip(x.asTerm) match
      case Apply(_, args) =>
        val vals = args.map { case NamedArg(_, v) => v; case v => v }
        vals.map(_.asExprOf[Int]).reduce((a, b) => '{ $a + $b })
      case _ => x
