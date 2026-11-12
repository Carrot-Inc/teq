package rules
import teq.fix.*
object Acc extends scala.collection.mutable.ArrayBuilder[Int]
final class ObjectExtendsBuilder extends SyntacticRule("ObjectExtendsBuilder"):
  override def fix(using doc: SyntacticDocument): Patch =
    Acc += 1
    Patch.lint(Diagnostic("n", (Acc.result().length).toString, doc.tree.pos))
