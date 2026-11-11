package ped

// A selection on a member the type lacks leaves the operator's application over it unresolved: the products build
// reports the error, as scalac 3.8.4 does (E008), where the capture's walk once met the unresolved application and
// panicked ("resolved selection", capture_census.rs). A library inline call teq cannot expand is such an error at
// its call (the workers exports helper), whose result an operator applies.
object Use:
  def c: String = pec.Lib.s.nosuch + "b"
