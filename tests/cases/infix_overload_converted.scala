// An overloaded left operand of `&&` whose result has the operator only through an implicit conversion.
import scala.language.implicitConversions
trait Trace
object Trace { given Trace = new Trace {} }
class C
class D { def &&(x: Int): Int = x }
implicit def convert(c: C): D = new D
object O:
  def f(implicit t: Trace): C = new C
  def f(n: Int): C = new C
@main def run(): Unit = println(O.f && 6)
