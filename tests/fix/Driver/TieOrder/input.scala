/*
rules = ["file:../../support/ReplaceAndImport.scala"]
*/
// The rule's replacement of the source and its added import both start at offset zero: dotty's order (a stable sort
// by start, the replacement first as made) puts the insertion inside the replacement, which is refused.
object Original
