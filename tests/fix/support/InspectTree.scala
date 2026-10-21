import teq.fix.*
import scala.collection.mutable.ListBuffer

/** The tree's contract (docs/FIX.md, "Tree"), the design pass's probe in both orders of access: the pre-order
  * traversal before and after an extraction, an extraction before any traversal, parents, equal handles and their
  * hashes, typed patterns, a nested pattern that fails, `traverse`; a lint each at the token `marker`. */
final class InspectTree extends SyntacticRule("InspectTree"):
  override def fix(using doc: SyntacticDocument): Patch =
    val at = doc.tokens.find(_.text == "marker").map(_.pos).getOrElse(doc.tree.pos)
    def walk = doc.tree.collect { case t => t.kind + ":" + t.syntax }
    def extract = doc.tree.collect { case Term.ApplyInfix.Initial(l, op, ts, rs) => l.syntax + op.value + ts.size + rs.map(_.syntax).mkString(",") }
    val extractedFirst = extract
    val before = walk
    val infix = doc.tree.collect { case t: Term.ApplyInfix => t }
    val again = doc.tree.collect { case t: Term.ApplyInfix => t }
    val identity = infix.map(a => "parent=" + a.lhs.parent.contains(a) + " equal=" + (a.lhs == a.children.head) + " hash=" + (a.lhs.hashCode == a.children.head.hashCode))
    val extracted = extract
    val nested = doc.tree.collect { case Term.ApplyInfix.Initial(_, Term.Name("nope"), _, _) => 1 }
    val typed = doc.tree.collect { case t: Term.ApplyInfix if t.op.value == "+" => t.syntax }
    val after = walk
    val visited = ListBuffer.empty[String]
    doc.tree.traverse(t => visited += t.kind)
    val shown = List(
      "nodes=" + before.size + " same=" + (before == after) + " traverse=" + (visited.toList == before.map(_.takeWhile(_ != ':'))),
      "extractedFirst=" + extractedFirst.mkString(";") + " extracted=" + extracted.mkString(";") + " equalHandles=" + (infix == again),
      "identity=" + identity.mkString(";"),
      "nested=" + nested.size + " typed=" + typed.mkString(";"),
      "root=" + doc.tree.kind + " parent=" + doc.tree.parent.isEmpty
    ) ++ before
    shown.map(s => Patch.lint(Diagnostic("tree", s, at))).asPatch
