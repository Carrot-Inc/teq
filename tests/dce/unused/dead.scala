// Nothing reads a val of this file, so its initialiser is left out.
package dce

val deadVal: Int =
  println("dead.scala initialised")
  7
