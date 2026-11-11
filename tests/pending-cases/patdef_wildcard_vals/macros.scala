import scala.quoted.*
object Shape:
  /** The names of the trees `e` binds, in order: each `Bind` and each `ValDef`, read off the tree's structure. */
  inline def names(inline e: Any): String = ${ namesImpl('e) }
  def namesImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val structure = e.asTerm.show(using Printer.TreeStructure)
    val found = """(Bind|ValDef)\("([^"]*)"""".r.findAllMatchIn(structure).map(m => s"${if m.group(1) == "Bind" then "bind" else "val"} ${mask(m.group(2))}")
    Expr(if found.isEmpty then "(none)" else found.mkString(", "))

  /** The names of the fields `T` declares. */
  inline def fields[T]: String = ${ fieldsImpl[T] }
  def fieldsImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val found = TypeRepr.of[T].typeSymbol.declaredFields.map(f => mask(f.name))
    Expr(if found.isEmpty then "(none)" else found.mkString(", "))

  // A fresh name's number is masked (`$N$`, `_$N`): scalac compiles a unit that calls a macro of the same run again
  // (CompilationUnit.suspend), its fresh names' counter going on, so the shape alone is compared.
  private def mask(n: String): String = n.replaceAll("""^\$[0-9]+\$$""", "\\$N\\$").replaceAll("""^_\$[0-9]+$""", "_\\$N")
