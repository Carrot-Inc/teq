package ped

// A selection on a member the type lacks
// leaves the operator's application over it
// unresolved: the products build is to report
// the error, as scalac 3.8.4 does (E008).
object Use:
  def c: String = pec.Lib.s.nosuch + "b"
