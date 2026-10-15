// Two implicit-only alternatives told apart by the infix operator selected on their results.
trait Base
trait Sub extends Base
object Evidence { given Sub = new Sub {} }
class C { def &&(n: Int): Int = n; def ++(n: Int): Int = n }
object O:
  def f(using x: Base): C = new C
  def f(using x: Sub): Int = 42
import Evidence.given
@main def run(): Unit = println(O.f ++ 6)
