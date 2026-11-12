// An overriding `tracked` parameter (experimental modularity), which RefChecks
// (1.12) leaves alone (`!member.is(Tracked)`) behind a private parameter passed
// another value: scalac 3.8.4 accepts it with -Werror. Pinned to a modularity part.
import scala.language.experimental.modularity
import scala.language.future
class A(val s: Int)
class B(s: Int) extends A(s + 1)
class C(override tracked val s: Int) extends B(s)
@main def run(): Unit =
  val c: C = C(3)
  println(c.s)
