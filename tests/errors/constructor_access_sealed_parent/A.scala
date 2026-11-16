// A sealed class is extended in its own source file alone, by an anonymous class of an empty body as by any
// other: scalac's E112 "Cannot extend sealed class Sea in a different source file" (B.scala); in its own file
// the creation stands, from another package of the file too. A sealed parent typed ahead and refused for its
// constructor is told of that alone, and one whose default is left out of E112 alone.
package lib
sealed class Sea(val x: Int)
sealed class SeaP protected (val x: Int)
sealed class SG[T] protected (val a: T)
sealed class SG0[T] protected ()
sealed class SD protected (val a: Int = 1)
object Same:
  val s1 = new Sea(1) {}
  val s2 = new SeaP(2) {}
