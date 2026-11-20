import scala.quoted.*

// A macro reads java.io.File's separators at compile time. Where the std's JavaScript layer is not loaded (the JVM
// target, JavaScript over scala-library), java.io.File is the JDK's class file, whose four static fields the
// interpreter's natives give as the host has them; elsewhere the std's `File` gives them. scalac's macro, run by a JVM
// on this machine, prints `/|/|:|:` here.
object Macros:
  inline def separators: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    Expr(java.io.File.separator + "|" + java.io.File.separatorChar + "|" + java.io.File.pathSeparator + "|" + java.io.File.pathSeparatorChar)
