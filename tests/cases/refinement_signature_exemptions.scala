// A structural member whose erased parameters are not the refinement's conforms where the refined
// type's class declares the member (the call is no reflective one) or where the refined type is a
// `Selectable.WithoutPreciseParameterTypes`, whose `applyDynamic` dispatches by name
// (`TypeComparer.hasMatchingMember`'s `sigsOK`).
trait B[A] { def f(x: A): Any }
class C extends B[String] { def f(x: String): String = "parent " + x }
def parent(b: B[String] { def f(x: String): String }): String = b.f("declared")

class Dyn extends scala.Selectable.WithoutPreciseParameterTypes:
  def applyDynamic(name: String)(args: Any*): Any = "marker " + name + "(" + args.mkString(", ") + ")"
class M[A] extends Dyn:
  def f(x: A): String = "unused"
type Marked = Dyn { def f(x: String): String }
def marked(m: Marked): String = m.f("dynamic")

@main def run(): Unit =
  println(parent(new C))
  println(marked(new M[String]))
