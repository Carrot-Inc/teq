// A bare sibling extension in the body of a right-associative extension of a group with a trailing using clause:
// it is selected on the group's receiver.
class D
final class C(val v: String)
given D = D()
extension (c: C)(using d: D)
  def tag: String = c.v
  def %:(g: String): String = g + tag
@main def run(): Unit = println(C("c") %: "g")
