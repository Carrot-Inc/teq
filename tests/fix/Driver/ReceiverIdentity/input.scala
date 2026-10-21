/*
rules = ["file:../../support/ReceiverIdentity.scala"]
*/
// `collect` and `traverse` visit their receiver as itself (reference identity), the root's and an inner node's.
object O { val marker = 1 + 2; val y = List(3) } // assert: ReceiverIdentity.identity
