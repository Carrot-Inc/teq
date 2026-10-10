// A macro calls java.util.Objects' defaulting members, whose JDK bodies the interpreter runs as the
// std's for every target's build, the JVM's included.
import scala.quoted.*

object Macros:
  inline def defaults: String = ${ defaultsImpl }

  def defaultsImpl(using Quotes): Expr[String] =
    val picked = java.util.Objects.requireNonNullElse[String](null, "fallback")
    val kept = java.util.Objects.requireNonNullElseGet[String]("kept", () => "made")
    val shown = java.util.Objects.toString(null, "none")
    val index = java.util.Objects.checkIndex(1, 2)
    Expr(s"$picked $kept $shown $index")
