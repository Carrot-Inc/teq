// A parameterised alternative normalized to its function type, whose `andThen` the selection names
// (`ProtoTypes.normalize`): the implicit-only alternative's result lacks it.
trait T
object T { given T = new T {} }
class C { def n: Int = 7 }
object O:
  def f(using T): C = new C
  def f(n: Int): C = new C
@main def run(): Unit = println(O.f.andThen((c: C) => c.n)(1))
