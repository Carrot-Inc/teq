// jars: bundle-lib
import scala.quoted.*

// A macro reading a resource bundle of the class path at expansion time, as scalactic's
// `Resources` does: the interpreter reads the `.properties` file as `java.util.Properties`
// does, escapes, continuation lines and both separators included.
object Macros:
  inline def message(inline key: String): String = ${ messageImpl('key) }
  def messageImpl(key: Expr[String])(using Quotes): Expr[String] =
    val bundle = java.util.ResourceBundle.getBundle("msgs")
    Expr(bundle.getString(key.valueOrAbort))
