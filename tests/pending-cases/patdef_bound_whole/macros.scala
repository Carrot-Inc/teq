import scala.quoted.*
object Shape:
  /** The names of the trees `e` binds, in order: each `Bind` and each `ValDef`, read off the tree's structure. */
  inline def names(inline e: Any): String = ${ namesImpl('e) }
  def namesImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val structure = e.asTerm.show(using Printer.TreeStructure)
    // A unique name's number is masked (`$N$`): scalac compiles a unit that calls a macro of the same run again
    // (CompilationUnit.suspend), its fresh names' counter going on, so the shape alone is compared.
    val found = """(Bind|ValDef)\("([^"]*)"""".r.findAllMatchIn(structure).map(m => s"${if m.group(1) == "Bind" then "bind" else "val"} ${m.group(2).replaceAll("""^\$[0-9]+\$$""", "\\$N\\$")}")
    Expr(found.mkString(", "))
