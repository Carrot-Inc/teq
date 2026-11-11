// A downstream of a/ as tests/modules builds it. a/'s bodies apply `+` to an operand ascribed an object's alias of Int,
// imported (`(x: T) + 1`) and by its path (`(x: O.I.T) + 1`). scalac 3.8.4 prints the .expected. teq withholds the
// bodies from a/'s products ("the operator + of a receiver"; in the lean products `plain` as "the operand of +"): the
// writer's primitive operator takes the alias for no primitive class. So b/'s build over the products fails at the
// inline call (tests/modules' build). The imported form fails the same before the SELECTtpt port (reviewer #14 on
// tip34); the selected form panicked there. Found by reviewer #14 of the workers exports helper (push 9).
package aob

@main def run(): Unit =
  println(aoa.A.inc(1))
  println(aoa.A.incSel(2))
  println(aoa.A.plain(3))
