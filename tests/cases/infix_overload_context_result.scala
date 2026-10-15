// Alternatives whose results are context functions, normalized through them (`ProtoTypes.normalize` follows a
// context function's result): `U ?=> C` has `++` through `C`, `T ?=> Int` has none.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def ++(n: Int): Int = n }
object O:
  def f(using T): U ?=> C = new C
  def f(using U): T ?=> Int = 0
@main def run(): Unit = println(O.f ++ 7)
