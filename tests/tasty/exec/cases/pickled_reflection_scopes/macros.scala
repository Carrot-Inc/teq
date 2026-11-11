import scala.quoted.*

object ReflectionScopes:
  inline def direct(inline value: Any): Boolean = ${ directImpl('value) }
  inline def nested(inline value: Any): Boolean = ${ nestedImpl('value) }
  inline def staged(inline value: Any): Boolean = ${ stagedImpl('value) }

  def isCall(using q: Quotes)(tree: q.reflect.Term): Boolean =
    import q.reflect.*
    tree match
      case Apply(_, _) => true
      case _ => false

  def directImpl(value: Expr[Any])(using q: Quotes): Expr[Boolean] =
    import q.reflect.*
    Expr(isCall(value.asTerm))

  def nestedImpl(value: Expr[Any])(using q: Quotes): Expr[Boolean] =
    import q.reflect.*
    def inspect(tree: Term): Boolean = tree match
      case Apply(_, _) => true
      case _ => false
    Expr(inspect(value.asTerm))

  def stagedImpl(value: Expr[Any])(using Quotes): Expr[Boolean] =
    '{
      val enabled = true
      ${
        import quotes.reflect.*
        val tree: Term = value.asTerm
        val call = tree match
          case Apply(_, _) => true
          case _ => false
        Expr(call)
      } && enabled
    }

  def left(q: Quotes)(r: Quotes)(tree: q.reflect.Term): q.reflect.Term =
    import q.reflect.*
    def identity(value: Term): Term = value
    identity(tree)

  def right(q: Quotes)(r: Quotes)(tree: r.reflect.Term): r.reflect.Term =
    import r.reflect.*
    def identity(value: Term): Term = value
    identity(tree)

  def leftPrinter(q: Quotes)(r: Quotes)(printer: q.reflect.Printer[q.reflect.Tree]): q.reflect.Printer[q.reflect.Tree] = printer
  def rightPrinter(q: Quotes)(r: Quotes)(printer: r.reflect.Printer[r.reflect.Tree]): r.reflect.Printer[r.reflect.Tree] = printer
