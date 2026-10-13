// An override of a member beside `Any`'s of its name takes the name its root goes by in the output,
// which is named apart from `Any`'s (`dispatch_name`) after the override is checked: a call through
// the root reaches the override, and the runtime's `toString` stays `Any`'s or the rule's.
abstract class C0:
  def toString(x: Int): String

class D0 extends C0:
  def toString(x: Int): String = "d" + x

@main def run(): Unit =
  val c: C0 = D0()
  println(c.toString(1))
  println(c.toString().take(3))
