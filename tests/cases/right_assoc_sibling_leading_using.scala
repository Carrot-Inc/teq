// A bare sibling extension in the body of an extension group whose first clause is a using clause: it is selected
// on the group's receiver (`c.tag`). scalac 3.8.4 refuses this program (E008, "value tag is not a member of B":
// Typer.leadParamOf takes the using parameter `b` as the receiver and tries `tag(b)(b)`).
class B
final class C(val v: String)
given B = B()
extension (using b: B)(c: C)
  def tag: String = c.v
  def other: String = "o" + tag
@main def run(): Unit = println(C("c").other)
