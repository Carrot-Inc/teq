import scala.quoted.*

trait Show:
  def show: String

object M:
  inline def make[T]: T = ${ makeImpl[T] }
  inline def nested[T]: Show = ${ nestedImpl[T] }

  def makeImpl[T: Type](using Quotes): Expr[T] =
    import quotes.reflect.*
    val comp = TypeRepr.of[T].typeSymbol.companionModule
    Select.unique(Ref(comp), "apply").appliedTo(Literal(IntConstant(1))).asExprOf[T]

  def nestedImpl[T: Type](using Quotes): Expr[Show] =
    import quotes.reflect.*
    val comp = TypeRepr.of[T].typeSymbol.companionModule
    val made = Select.unique(Ref(comp), "apply").appliedTo(Literal(IntConstant(2))).asExprOf[T]
    '{ new Show { def show: String = $made.toString } }
