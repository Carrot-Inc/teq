// A user method spelled as the getter of a default an overridden member declares: the override's call that leaves
// the argument out runs the base's default; scalac prints 107, 107, then 99.
class Base { def m(x: Int = 7): Int = x }
class Sub extends Base {
  override def m(x: Int): Int = x + 100
  def `m$default$1`(s: String): Int = 99
}
@main def run(): Unit = {
  val b: Base = new Sub
  println(b.m())
  println(new Sub().m())
  println(new Sub().`m$default$1`("s"))
}
