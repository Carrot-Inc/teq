import teq.fix.*

/** Reports the symbol of each name written in an annotation, where it has one (tests/fix/Driver/AnnotationArguments,
  * tests/fix/Driver/AnnotationArgumentsLibrary). */
final class AnnotationNames extends SemanticRule("AnnotationNames"):
  override def fix(using doc: SemanticDocument): Patch =
    doc.tree.collect { case annotation: Mod.Annot =>
      annotation.collect { case name: Name if !name.symbol.isNone =>
        Patch.lint(Diagnostic("at", name.symbol.value, name.pos, severity = LintSeverity.Warning))
      }.asPatch
    }.asPatch
