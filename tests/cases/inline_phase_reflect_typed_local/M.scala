// A macro reads the typed tree of a definition whose plain inline call the inlining phase expanded
// before the macro's run: it finds the call, as dotty's `defTree` (set by `PostTyper`) keeps it
// (scalac prints true).
import scala.quoted.*
object M:
  inline def plain: Int = 42
  inline def inspect: Boolean = ${impl}
  def impl(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    var owner = Symbol.spliceOwner
    while owner.name != "test" && owner != Symbol.noSymbol do owner = owner.owner
    var found = false
    val scan = new TreeTraverser:
      override def traverseTree(tree: Tree)(owner: Symbol): Unit =
        if tree.symbol.name == "plain" then found = true
        traverseTreeChildren(tree)(owner)
    scan.traverseTree(owner.tree)(owner)
    Expr(found)
