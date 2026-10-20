import scala.quoted.*
object PRInspect:
  inline def plain: Int = 42
  def targetValue: Int = plain
  inline def inspect: Boolean = ${impl}
  def impl(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    val s = '{ PRInspect.targetValue }.asTerm.underlyingArgument.symbol
    var found = false
    val scan = new TreeTraverser:
      override def traverseTree(t: Tree)(o: Symbol): Unit =
        if t.symbol.name == "plain" then found = true
        traverseTreeChildren(t)(o)
    scan.traverseTree(s.tree)(s)
    Expr(found)
