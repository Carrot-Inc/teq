// A bare right-associative sibling called in the body of another of a group with a leading using clause: the
// group's receiver is the call's qualifier, so it is the sibling's own parameter and the argument its receiver.
// scalac 3.8.4 refuses this program (E007, "Found: String => String"): Typer.leadParamOf takes the method's own
// parameter for each method's receiver, which are not the same parameter, and `%:("y")` stays a function.
class B
given B = B()
extension (using b: B)(c: String)
  def %:(f: String): String = "f=" + f + ",c=" + c
  def #:(f: String): String = %:("y")
@main def run(): Unit = println("x" #: "C")
