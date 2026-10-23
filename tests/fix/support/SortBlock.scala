import teq.fix.*

/** A rule that sorts the leading imports of the source, or of its one package clause: every token from the first
  * import's first to the last import's last removed, the statements, sorted and each once, added left of the first
  * token; a block already so left as it is (tests/fix/Merge). */
final class SortBlock extends SyntacticRule("SortBlock"):
  override def fix(using doc: SyntacticDocument): Patch =
    val imports = leading(doc.tree.asInstanceOf[Source].stats)
    val sorted = imports.map(_.syntax).distinct.sorted
    if imports.isEmpty || sorted == imports.map(_.syntax) then Patch.empty
    else
      val first = imports.head.tokens.head
      Patch.removeTokens(doc.tokens.filter(t => t.start >= first.start && t.end <= imports.last.pos.end)) +
        Patch.addLeft(first, sorted.mkString("\n"))

  private def leading(stats: List[Tree]): List[Import] = stats match
    case (p: Pkg) :: Nil => leading(p.stats)
    case _ => stats.takeWhile(_.isInstanceOf[Import]).map(_.asInstanceOf[Import])
