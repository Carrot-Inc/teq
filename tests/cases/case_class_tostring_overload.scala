// A method beside `Any`'s `toString`, `hashCode` and `equals` with other parameters overloads
// them: the class keeps the rule's `toString` (dotty's `existingDef` matches the signature), and
// the overload is a member of its own.
case class C(x: Int):
  def toString(n: Int): String = "overload" + n
class D:
  def hashCode(n: Int): Int = n
  override def toString = "D"

@main def run(): Unit =
  println(C(1))
  println(C(1).toString(2))
  println(new D)
  println(new D().hashCode(5))
