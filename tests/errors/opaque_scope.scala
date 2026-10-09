// expect: 10:14: error: type mismatch: found String, required A
// expect: 11:25: error: type mismatch: found A, required String
// expect: 15:18: error: the underlying type Int does not conform to the bound String
// expect: 3 errors found
opaque type A = String
val ok: A = "abc"
object A:
  def make(s: String): A = s
object obj:
  val y: A = "abc"
  def f(a: A): String = a
def top(a: A): String = a
def anon(a: A): Runnable = new Runnable:
  def run(): Unit = println(a: String)
opaque type B <: String = Int
trait Runnable:
  def run(): Unit
