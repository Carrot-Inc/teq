/*
rules = ["file:../../support/DropImportees.scala", "file:../../support/AtomicLineAfterImport.scala"]
*/
// A suppressed atomic patch changes no other rule's rewrite: the line break the suppressed removal would take is not
// the one the import removed whole loses (the import conversion's mask is of the edits that survive suppression,
// scalafix's bytes, the same as with the removing rule alone; the pass over 0f91053d7). The import left keeps the
// anchors used in the second round, a block comment's anchor the added line joins.
object Names { val a = 1; val b = 2 }
/* scalafix:off AtomicLineAfterImport */
import Names.a
import Names.b
/* scalafix:on AtomicLineAfterImport */
object O
