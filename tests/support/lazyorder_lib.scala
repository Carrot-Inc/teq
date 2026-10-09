// The jar of tests/split.sh's lazy-order check: an object's lazy vals, which a program reads in
// either order and whose module is the same bytes whichever it reads first.
package lazyorderlib

object Library:
  lazy val a: Int = { println("a"); 1 }
  lazy val b: Int = { println("b"); 2 }
  lazy val c: Int = { println("c"); 3 }
