// A bare sibling extension in the body of a right-associative extension: it is selected on the group's receiver.
final class C(val v: String)
extension (c: C)
  def tag: String = c.v
  def %:(f: String): String = f + tag
@main def run(): Unit = println(C("c") %: "f")
