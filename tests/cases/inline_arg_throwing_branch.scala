// A branch that throws joins to the other branch: the parameter bound to `if flag then new C
// else throw ..` has the class's type, and the abstract inline call on it reaches the class.
trait B:
  inline def apply[T](inline n: Int): String
class C extends B:
  inline def apply[T](inline n: Int): String = scala.compiletime.constValue[T].toString + n
class D extends B:
  inline def apply[T](inline n: Int): String = "d" + n
inline def f(x: B): String = x["ok"](7)
def pick(flag: Boolean): C = if flag then new C else throw new Exception("bad")
// A context function applied in a branch has its result's type there (`fn: Int ?=> C` is a
// `C` in the join, not the `B` expected).
@main def run(): Unit =
  val flag = true
  println(f(if flag then new C else throw new Exception("bad")))
  println(f(flag match { case true => new D; case false => throw new Exception("bad") }))
  println(f({ if !flag then throw new Exception("bad"); new C }))
  println(f(if flag then new C else sys.error("bad")))
  given Int = 1
  val fn: Int ?=> C = new C
  println(f(if flag then fn else throw new Exception("bad")))
  println(f(flag match { case true => fn; case false => throw new Exception("bad") }))
