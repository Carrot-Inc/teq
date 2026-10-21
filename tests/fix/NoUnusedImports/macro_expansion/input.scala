/*
rules = NoUnusedImports
*/
// Invented from tests/warnings/probes/macro_expansion (no test of dotty's holds it): scalac 3.8.4 counts `import p.*`
// used through the derived macro's expansion, which teq's analysis does not see yet; NoUnusedImports leaves the import
// in a file where an expansion was made and says so (docs/FIX.md, "NoUnusedImports").
package p

import core.*
import p.* // assert: NoUnusedImports.unverified

// A macro's expansion names the companion of `Level`: scalac resolves it through this file's
// imports, the expansion's source the macro's file, so `Level` counts as a member of the package
// from another file and `import p.*` is used; without `derives En` it is not.
enum Level derives En:
  case A, B

val e: En[Int] = null
