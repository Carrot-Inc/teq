package rules
import teq.fix.*
object Seen extends scala.collection.mutable.HashMap[String, Int]
final class ObjectExtendsMap extends SyntacticRule("ObjectExtendsMap"):
  override def fix(using doc: SyntacticDocument): Patch =
    Seen(doc.input.path) = 1
    Patch.lint(Diagnostic("n", (Seen.size).toString, doc.tree.pos))
