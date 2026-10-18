import scala.quoted.*

// A macro asks reflection for a class of a package whose name encoding changes: the root package keys the package by
// its encoded name (dotty's PackageScope keys by name.mangled), and `Symbol.requiredClass` finds it by its source
// spelling through that key, as scalac 3.8.4 prints `C`.
object Macros:
  inline def name: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(Symbol.requiredClass("a-b.C").name)
