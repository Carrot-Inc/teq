package rules
import teq.fix.*
final class CostHelperBuffer extends SyntacticRule("CostHelperBuffer"):
  private def gather(text: String): List[String] =
    val out = scala.collection.mutable.ListBuffer.empty[String]
    var i = 0
    while i < text.length do
      out += text.substring(i, i + 1)
      i += 1
    out.toList
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (gather(doc.input.path).length).toString, doc.tree.pos))
