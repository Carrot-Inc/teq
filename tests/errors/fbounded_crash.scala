// expect: type mismatch: found Int, required T
// expect: method apply in class C does not take parameters
trait Ord[T <: Ord[T]]
def f[T <: Ord[T]](t: T): T = t
def test = f(1)

class C:
  def apply: C = this
def t2 = (new C)(22)

@main def run(): Unit = println(1)
