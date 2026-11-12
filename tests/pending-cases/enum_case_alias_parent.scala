// An enum case whose parent names its enum through an
// alias (`type EA = E`, `extends EA(s)`): dotty types
// the parent (`typedParent`) and reads its class, so
// the case passes `s` to the enum, as scalac 3.8.4
// runs it. Queued with the enum desugaring's part.
enum E(val s: Int):
  case C(override val s: Int) extends EA(s)
type EA = E
@main def run(): Unit = println(E.C(3).s)
