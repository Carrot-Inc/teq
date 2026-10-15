// Compiled with Scala 3.8.4: a macro whose body matches a block's statements with the reflection API's extractors,
// `Import` and `Export` among them, which the pickle reads through their `TypeTest` givens
// (`q.reflect.ImportTypeTest(q.reflect.Import(_, _))`, as ultraviolet's `CreateShaderAST.walkStatement` reads them).
package fix.rstats

import scala.quoted.*

object StatementKinds:
  inline def kinds(inline body: Any): List[String] = ${ kindsImpl('body) }

  def kindsImpl(body: Expr[Any])(using Quotes): Expr[List[String]] =
    import quotes.reflect.*
    def walk(t: Term): List[String] = t match
      case Inlined(_, _, inner) => walk(inner)
      case Block(stats, _) => stats.map(kind)
      case _ => Nil
    def kind(s: Statement): String = s match
      case Import(_, selectors) => s"import of ${selectors.length}"
      case Export(_, _) => "export"
      case ValDef(name, _, _) => s"val $name"
      case DefDef(name, _, _, _) => s"def $name"
      case _ => "other"
    Expr(walk(body.asTerm))
