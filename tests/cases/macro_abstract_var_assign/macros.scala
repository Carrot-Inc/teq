import scala.quoted.*

// An assignment through an abstract var is an `Assign` of the var's selection to a macro, as in
// scalac's tree, and an `Assign` a macro builds of such a selection calls the setter.
object Probe:
  inline def assignment(inline e: Any): String = ${ assignmentImpl('e) }
  def assignmentImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val text = e.asTerm.underlyingArgument match
      case Assign(Select(_, name), Literal(c)) => s"Assign($name, ${c.value})"
      case Assign(Ident(name), Literal(c)) => s"Assign($name, ${c.value})"
      case Assign(_, _) => "Assign"
      case other => "not an Assign"
    Expr(text)

  inline def setTo(inline target: Int, inline value: Int): Unit = ${ setToImpl('target, 'value) }
  def setToImpl(target: Expr[Int], value: Expr[Int])(using Quotes): Expr[Unit] =
    import quotes.reflect.*
    Assign(target.asTerm.underlyingArgument, value.asTerm).asExprOf[Unit]
