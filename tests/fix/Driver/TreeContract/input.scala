/*
rules = ["file:../../support/InspectTree.scala"]
*/
// The tree's contract (docs/FIX.md, "Tree"): the design pass's probe (fix-speed's), and after another document (B.scala).
object O { val marker = 1 + 2; val y = List(3) } // assert: InspectTree.tree
