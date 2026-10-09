import scala.quoted.*

// A macro reads the build's environment through `System.getenv()` and scala-library's
// `sys.env`, as scalac's JVM gives it (levsha's optimizer checks a variable there).
object Macros:
  inline def envSeen: String = ${ envSeenImpl }
  def envSeenImpl(using Quotes): Expr[String] =
    val all = System.getenv()
    Expr(s"${all.containsKey("PATH")} ${sys.env.contains("PATH")} ${sys.env.get("TEQ_UNSET_VARIABLE_42")}")
