// An implicit-only alternative beside a one-parameter one under a dotted `&&`.
trait Trace
object Trace { given Trace = new Trace {} }
class C { def &&(n: Int): Int = n }
object O:
  def f(using x: Trace): C = new C
  def f(n: Int): C = new C
@main def run(): Unit = println(O.f.&&(6))
