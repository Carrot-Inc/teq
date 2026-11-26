// A bare sibling extension in the body of a right-associative extension whose own first clause is a using clause,
// so that its clauses are not swapped: it is selected on the group's receiver (`c.tag`). scalac 3.8.4 eta-expands
// `tag` here and prints a function where the receiver's value is written: Typer.leadParamOf takes the trailing using
// parameter `d` for the receiver.
class D
class X
final class C(val v: String)
given D = D()
given X = X()
extension (c: C)(using d: D)
  def tag: String = c.v
  def %:(using x: X)(g: String): String = g + tag
@main def run(): Unit = println(C("c").%:("g"))
