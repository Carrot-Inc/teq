// `Symbol.tree` of a member of a class of a block has no right-hand side: dotty retains a definition's
// tree where its immediate owner is a term (`Symbols.retainsDefTree`), which the class's is and its
// member's is not; the class's own tree, retained, shows the member with its body (scalac prints false
// and true).
@main def run(): Unit =
  class Local:
    def value: Int = 1
  println(M.memberHasRhs[Local])
  println(M.classTreeHasRhs[Local])
