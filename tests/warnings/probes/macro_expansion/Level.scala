package p

import core.*
import p.*

// A macro's expansion names the companion of `Level`: scalac resolves it through this file's
// imports, the expansion's source the macro's file, so `Level` counts as a member of the package
// from another file and `import p.*` is used; without `derives En` it is not.
enum Level derives En:
  case A, B

val e: En[Int] = null
