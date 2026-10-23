import teq.fix.*

/** Lints the shared file at its first visit alone: a project rule's state, which the jobs' visits change in turn. */
final class Visit extends SyntacticRule("Visit"):
  var visits = 0
  override def fix(using doc: SyntacticDocument): Patch =
    if doc.input.text.contains("object Shared") then
      visits += 1
      if visits == 1 then Patch.lint(Diagnostic("first", "first shared-file visit", doc.tree.pos)) else Patch.empty
    else Patch.empty
