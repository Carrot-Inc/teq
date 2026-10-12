// A macro's `Apply` gives a constructor's first clause, of one parameter, two arguments: dotty
// types the application (`TypeAssigner.assignType`) and reports the count at the expansion, the
// erroneous tree's next `Apply` and its cast silent.
import scala.quoted.*
object WrongMacro:
  inline def make[C]: C = ${ impl[C] }
  def impl[C: Type](using Quotes): Expr[C] =
    import quotes.reflect.*
    val ctor = Select(New(TypeTree.of[C]), TypeRepr.of[C].typeSymbol.primaryConstructor)
    val first = Apply(ctor, List('{ new Coll }.asTerm, '{ "x" }.asTerm))
    Apply(first, Nil).asExprOf[C]
