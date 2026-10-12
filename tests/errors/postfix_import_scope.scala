// expect: 9:24: error: expected end of statement, found identifier
// A language import enables its feature until the end of the statements it stands among, as dotty
// restores `languageImportContext` after a template's, a block's and an extension's statements: a
// sibling object without the import has no postfix operators.
object Enabled:
  import scala.language.postfixOps
  val one = List(1) size
object Disabled:
  val two = List(1, 2) size
@main def run(): Unit = println(Disabled.two)
