// jars: fixtures fixtures-rdecl
// targets: js interp jvm
// Transparent expansions a library body holds: two methods of one class, one
// expanding the other, on a stable and a fresh receiver, and an argument inlined from the
// caller's scope, whose `INLINED` has no call.
import fix.rdecl.*
import fix.reader.*

@main def main(): Unit =
  println(RdCalls.expansions(RdTraces()))
  println(RdCalls.outerScope(RdTraces()))
