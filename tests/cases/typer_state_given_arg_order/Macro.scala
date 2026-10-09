import scala.quoted.*

object M:
  private var log: List[String] = Nil
  def note(s: String)(using Quotes): Expr[String] =
    log = s :: log
    Expr(log.reverse.mkString(","))
  inline def a: String = ${ note("a") }
  inline def pgv: String = ${ note("pg") }
