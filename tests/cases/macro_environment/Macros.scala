package envlib
import scala.quoted.*

object Env:
  inline def probe(inline key: String): String = ${ probeImpl('key) }

  def probeImpl(key: Expr[String])(using Quotes): Expr[String] =
    val value = System.getenv(key.valueOrAbort)
    Expr(if value == null then "unset" else "set")
