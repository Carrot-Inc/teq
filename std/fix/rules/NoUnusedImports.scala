// teq's rule over the typer's unused-import analysis (`-Wunused:imports`, src/typer/unused.rs): every importee the
// analysis reports removed by dotty's edits (`CheckUnused.checkImports`, src/typer/unused_edits.rs), the importer
// with its last importee and the import with its last importer. In a document where an expansion was made, a macro's
// or an inline method's, scalac credits imports teq's analysis does not yet see (docs/FIX.md, "NoUnusedImports"):
// there each unused importee is reported as `NoUnusedImports.unverified` and left.
package teq.fix.rules

import teq.fix.*

final class NoUnusedImports extends SemanticRule("NoUnusedImports"):
  override def fix(using doc: SemanticDocument): Patch =
    val unused = doc.unusedImports
    if unused.isEmpty then Patch.empty
    else if doc.hasExpansions then
      unused.map { i =>
        Patch.lint(Diagnostic("unverified", "unused import, left: an expansion in this file may use it", i.pos, severity = LintSeverity.Warning))
      }.asPatch
    else unused.map(Patch.removeImportee).asPatch
