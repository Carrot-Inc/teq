// expect: 10:14: error: Found:    "abc"⏎Required: A
// expect: 11:25: error: Found:    A⏎Required: String
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
