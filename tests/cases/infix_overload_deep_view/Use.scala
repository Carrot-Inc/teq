// Neither alternative's result has `++`; the views pass deepens the selection, and only `C`'s
// conversion gives a `++` that takes the `Int` (`isCompatible`'s `viewExists`, ProtoTypes.scala).
import scala.language.implicitConversions
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C
class D
class CC { def ++(n: Int): Int = n }
class DD { def ++(s: String): Int = s.length }
given Conversion[C, CC] = _ => new CC
given Conversion[D, DD] = _ => new DD
object O:
  def f(using T): C = new C
  def f(using U): D = new D
@main def run(): Unit = println(O.f ++ M.arg)
