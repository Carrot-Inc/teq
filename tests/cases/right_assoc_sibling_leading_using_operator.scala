// A bare sibling extension in the body of a right-associative extension of a group with a leading using clause: it
// is selected on the group's receiver (`c.tag`). scalac 3.8.4 eta-expands `tag` here and prints a function where the
// receiver's value is written: Typer.leadParamOf takes the method's own parameter `f` for the receiver, and the
// sibling's receiver is not the same parameter.
class B
final class C(val v: String)
given B = B()
extension (using b: B)(c: C)
  def tag: String = c.v
  def %:(f: String): String = f + tag
@main def run(): Unit = println(C("c") %: "f")
