import teq.fix.*

// The document's tree as scalameta's `MetaDump.scala` prints a tree: a node a line, its kind, its start and its end, its
// children indented under it; one lint, at the definition `val marker`, else at the first import.
final class DumpLayout extends SyntacticRule("DumpLayout"):
  override def fix(using doc: SyntacticDocument): Patch =
    val out = new StringBuilder
    def walk(t: Tree, depth: Int): Unit =
      out ++= "  " * depth + t.kind + " " + t.pos.start + " " + t.pos.end + "\n"
      t.children.foreach(walk(_, depth + 1))
    walk(doc.tree, 0)
    val marker = doc.tree.collect { case t: Defn.Val if t.syntax.startsWith("val marker") => t.pos }.headOption
    val at = marker.orElse(doc.tree.collect { case t: Import => t.pos }.headOption).getOrElse(doc.tree.pos)
    Patch.lint(Diagnostic("layout", "BEGIN\n" + out.toString + "END", at))
