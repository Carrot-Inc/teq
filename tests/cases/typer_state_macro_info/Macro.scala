import scala.quoted.*

object Loud:
  inline def one(): Int = ${ oneImpl }
  def oneImpl(using Quotes): Expr[Int] =
    quotes.reflect.report.info("the macro ran")
    Expr(1)
