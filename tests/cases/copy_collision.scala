// A class's storage against a subclass's own field of any spelling (`Constructors.checkNoFieldClashes`): A's copy of
// its parameter `s`, which its deferred body reads, beside B's val `$s`.
class A(val s: Int):
  val saved = s
class B(override val s: Int, val $s: Int) extends A(s)
@main def run(): Unit =
  val b = B(2, 99)
  println(b.saved)
  println(b.$s)
