// jars: fixtures fixtures-rdecl
// targets: js interp jvm
// References of library bodies to overloads, as scalac resolved them: an
// overload of a class of another jar through an applied and a generic prefix, one alternative
// told apart by its target name, an object's two alternatives of one erasure, a method whose
// override the jar lost since the body was compiled (found in the parent), a local generic
// method, a class's own overloads reached by symbol, and overloads added to the jar since, which
// the erasure's qualified names, a generic array's bound and a value class's argument tell apart,
// on a receiver of an intersection type and over an array of a bounded wildcard.
import fix.rdecl.*
import fix.reader.*

@main def main(): Unit =
  println(RdCalls.applied(RdBox("v")))
  println(RdCalls.generic(RdBox(1), 2))
  println(RdCalls.targets)
  println(RdCalls.inherited(RdSquare()))
  println(RdCalls.localGeneric)
  println(RdLocal(7).self + ", " + RdLocal(8).other(RdLocal("o")))
  println(RdCalls.tokens)
  println(RdCalls.viaInter(new RdApi with RdMarker))
  println(RdCalls.boundedArray)
