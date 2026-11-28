// A user method spelled as a generic default's getter (`m$default$1` taking a String): the call that leaves the
// argument out runs the default, by the getter named apart from the method; scalac prints seven, then 99.
class C {
  def m[A](x: A = "seven".asInstanceOf[A]): A = x
  def `m$default$1`(s: String): Int = 99
}
@main def run(): Unit = {
  val c = new C
  println(c.m[String]())
  println(c.`m$default$1`("s"))
}
