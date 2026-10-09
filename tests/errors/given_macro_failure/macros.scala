import scala.quoted.*
trait Show[T]:
  def show(t: T): String
object Show:
  given Show[String] = s => s
  inline given derived[T]: Show[T] = ${ derivedImpl[T] }
  def derivedImpl[T: Type](using Quotes): Expr[Show[T]] =
    import quotes.reflect.*
    report.errorAndAbort("cannot derive Show for " + Type.show[T])
