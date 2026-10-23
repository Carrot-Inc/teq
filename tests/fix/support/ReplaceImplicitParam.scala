import teq.fix.*

/** Replaces each lambda parameter with an `implicit` modifier with its name, whole
  * (tests/fix/Driver/ReplaceImplicitParam). */
final class ReplaceImplicitParam extends SyntacticRule("ReplaceImplicitParam"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case p: Term.Param if p.mods.exists(_.isInstanceOf[Mod.Implicit]) => Patch.replaceTree(p, p.name.syntax)
    }.asPatch
