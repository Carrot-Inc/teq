import teq.fix.*

/** A rule that removes the tokens of each import statement written again later in its file (tests/fix/Merge). */
final class DedupImports extends SyntacticRule("DedupImports"):
  override def fix(using doc: SyntacticDocument): Patch =
    val imports = doc.tree.collect { case i: Import => i }
    imports.zipWithIndex.collect {
      case (i, k) if imports.drop(k + 1).exists(_.syntax == i.syntax) => Patch.removeTokens(i.tokens)
    }.asPatch
