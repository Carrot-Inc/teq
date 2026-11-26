// A bare right-associative sibling called in another extension's body: the group's receiver is the call's
// qualifier, so it is the sibling's own parameter, and the argument is the sibling's receiver.
extension (c: String)
  def %:(f: String): String = "f=" + f + ",c=" + c
  def other: String = %:("x")
@main def run(): Unit = println("C".other)
