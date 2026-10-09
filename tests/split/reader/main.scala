// jars: fixtures fixtures-rdecl
// The resident session over library bodies whose references the reader keeps
// (tests/split-watch.sh): an overloaded call of another jar, a local generic
// method, a nested pattern binder, and two files expanding a library class's transparent methods.
import fix.rdecl.*
import fix.reader.*

@main def main(): Unit =
  println(RdCalls.applied(RdBox("v")))
  println(RdCalls.localGeneric)
  println(RdCalls.nested(RdBox("n")) + " " + RdCalls.nested(RdBox(1)) + " " + RdCalls.nested(3))
  println(Ones.run + Twos.run)
