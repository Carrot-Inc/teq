import scala.quoted.*

// A macro reads the build's system properties through scala-library's `sys.props`, as scalac's
// JVM gives them (munit's `Location` takes a source's path relative to `user.dir`); a property
// that is not set is null through `apply`.
object Macros:
  inline def propsSeen: String = ${ propsSeenImpl }
  def propsSeenImpl(using Quotes): Expr[String] =
    val dir = sys.props("user.dir")
    Expr(s"${dir.startsWith("/")} ${dir == System.getProperty("user.dir")} ${sys.props("file.separator")} ${sys.props.get("teq.unset.property.42")} ${sys.props.contains("user.dir")} ${sys.props("teq.unset.property.42")}")
