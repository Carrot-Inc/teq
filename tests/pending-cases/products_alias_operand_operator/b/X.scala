// A downstream of a/ as tests/modules builds it. a/'s bodies
// apply `+` to an operand ascribed an object's alias of Int,
// imported (`(x: T) + 1`) and by its path (`(x: O.I.T) + 1`).
// scalac 3.8.4 prints the .expected, and b/'s build over a/'s
// products (tests/modules' build) is to print it too. Found
// by reviewer #14 of the workers exports helper (push 9).
package aob

@main def run(): Unit =
  println(aoa.A.inc(1))
  println(aoa.A.incSel(2))
  println(aoa.A.plain(3))
