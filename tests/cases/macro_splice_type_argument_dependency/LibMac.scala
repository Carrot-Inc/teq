package libmac
import scala.quoted.*

object LibMac:
  def expanded[T: Type](using Quotes): Expr[String] = Expr("expanded")
