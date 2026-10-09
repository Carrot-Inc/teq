import scala.quoted.*

// A `TreeMap` that leaves the trees as they are, over blocks holding a type definition:
// `TypeDef.copy` gives the tree back (the std's `TreeMap` needs `TypeBoundsTree.copy` as well).
object Macros:
  inline def mapped[A](inline body: A): A = ${ mappedImpl('body) }
  def mappedImpl[A: Type](body: Expr[A])(using Quotes): Expr[A] =
    import quotes.reflect.*
    val identity = new TreeMap {}
    identity.transformTerm(body.asTerm)(Symbol.spliceOwner).asExprOf[A]
