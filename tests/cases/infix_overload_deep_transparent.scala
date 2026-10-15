// The deeper selection types the operator's argument as dotty's `FunProto.typedArgs` types it (`typer.typed`,
// ProtoTypes.scala 508): the transparent call expanded, its `Int` choosing the alternative whose `&&` takes one.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
class C { def &&(n: Int): Int = n }
class D { def &&(s: String): Int = s.length }
object O:
  def f(using T): C = new C
  def f(using U): D = new D
transparent inline def arg: Any = 6
import E.given
@main def run(): Unit = println(O.f && arg)
