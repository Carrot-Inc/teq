// jars: fixtures
// A library's macro, read from its TASTy, matching a block's statements with the reflection API's `Import` and
// `Export` extractors, which its pickle reads through their `TypeTest` givens (`Quotes`' `ImportTypeTest` and
// `ExportTypeTest`), as ultraviolet's `CreateShaderAST.walkStatement` does.
import fix.rstats.StatementKinds

@main def run(): Unit =
  println(StatementKinds.kinds {
    val a = 1
    def f(x: Int) = x + a
    f(a)
  })
