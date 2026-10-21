/*
rules = ["file:../../support/Stale.scala"]
*/
// The rule keeps a token of the first document it reads and replaces it while it reads the other (`B.scala`): the
// run refuses the patch as the token's accessors refuse it, and writes neither file.
object A { val old = 1 }
