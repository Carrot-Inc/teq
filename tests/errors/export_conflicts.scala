// expect: 23:12: error: Conflicting definitions:
// expect: def x: String in object B at line 24 and
// expect: 27:12: error: Conflicting definitions:
// expect: final def x: String in object C at line 26 and
// expect: 30:12: error: Conflicting definitions:
// expect: def x: String in object Fine at line 29 and
// expect: 31:12: error: Conflicting definitions:
// expect: final def only: String in object Fine at line 30 and
// expect: 4 errors found
// Exports that clash with a member or with each other: each forwarder is a member of the object,
// and two that erase alike are a double definition, as scalac's E120 reports them (an export of a
// trait's concrete member, `G`'s, overrides nothing that forbids it).
object A:
  def x = "A.x"
  def only = "only A"
object R:
  def x = "R.x"
object Impl:
  def greet = "hello from Impl"
trait Greeter:
  def greet: String = "trait greet"
object B:
  export A.x
  def x = "B.x"
object C:
  export A.*
  export R.*
object Fine:
  def x = "own"
  export A.*
  export A.only
object G extends Greeter:
  export Impl.greet
