// The top-level vals of a file initialise together: reading `usedVal` runs `siblingVal` too.
package dce

val usedVal: Int =
  println("vals.scala initialised")
  40 + 2

val siblingVal: String =
  println("sibling initialised")
  "sibling"
