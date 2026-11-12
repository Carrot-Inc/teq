package rules
import teq.fix.*
final class VectorBuilderMember extends SyntacticRule("VectorBuilderMember"):
  private val acc = new scala.collection.immutable.VectorBuilder[String]()
  override def fix(using doc: SyntacticDocument): Patch =
    acc += doc.input.path
    Patch.lint(Diagnostic("n", (acc.result().size).toString, doc.tree.pos))
