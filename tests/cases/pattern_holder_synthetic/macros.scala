package demo.holders

import scala.quoted.*

object Inspect:
  /** The generated vals of the block, and how many of them reflection reports `Synthetic`. */
  inline def check(inline e: Any): String = ${ checkImpl('e) }
  /** The same of a class's fields. */
  inline def fields[T]: String = ${ fieldsImpl[T] }

  def checkImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    var generated = 0
    var synthetic = 0
    val walk = new TreeTraverser:
      override def traverseTree(t: Tree)(owner: Symbol): Unit =
        t match
          case v: ValDef if v.name.startsWith("$") =>
            generated += 1
            if v.symbol.flags.is(Flags.Synthetic) then synthetic += 1
          case _ => ()
        super.traverseTree(t)(owner)
    walk.traverseTree(e.asTerm)(Symbol.spliceOwner)
    Expr(s"$generated $synthetic")

  def fieldsImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val held = TypeRepr.of[T].typeSymbol.declaredFields.filter(_.name.startsWith("$"))
    Expr(s"${held.size} ${held.count(_.flags.is(Flags.Synthetic))}")
