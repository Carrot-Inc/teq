import scala.quoted.*

object M:
  private var log: List[String] = Nil
  def note(s: String)(using Quotes): Expr[String] =
    log = s :: log
    Expr(log.reverse.mkString(","))
  def noteWarn(s: String)(using Quotes): Expr[String] =
    log = s :: log
    quotes.reflect.report.warning("warn " + s)
    Expr(log.reverse.mkString(","))
  inline def a: String = ${ note("a") }
  inline def b: String = ${ note("b") }
  inline def pgv: String = ${ note("pg") }
  inline def warnv: String = ${ noteWarn("w") }
