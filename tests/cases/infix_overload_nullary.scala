// A parameterless alternative beside a parameterised one, the left operand of a user `&&`.
class C { def &&(x: Int): Int = x }
object O:
  def f: C = new C
  def f(n: Int): C = new C
@main def run(): Unit = println(O.f && 6)
