// A structural call goes to the `applyDynamic` its qualifier's `Selectable` has, after an implicit
// conversion where the qualifier is none, and the classes of the method's parameters are passed only
// when that `applyDynamic`'s second parameter is `Class[?]*` (dotty's `Dynamic.handleStructural`,
// `structuralCall`, `addClassOfs`): a `String*` there stays empty.
import scala.language.implicitConversions

class C
class Wrap(x: C) extends Selectable:
  def applyDynamic(name: String, tags: String*)(args: Any*): Any =
    tags.mkString(",") + ":" + args.mkString(",")
given Conversion[C, Wrap] = x => new Wrap(x)

class Typed extends Selectable:
  def selectDynamic(name: String): Any = "field " + name
  def applyDynamic(name: String, classes: Class[?]*)(args: Any*): Any =
    name + " " + classes.length + ":" + args.mkString(",")

def call(x: C { def f(a: Int): Any }): Any = x.f(3)
def typed(t: Typed { def g(a: Int, b: String): Any; val v: Any }): String = s"${t.g(1, "s")} ${t.v}"

@main def run(): Unit =
  println(call(new C().asInstanceOf[C { def f(a: Int): Any }]))
  println(typed(new Typed().asInstanceOf[Typed { def g(a: Int, b: String): Any; val v: Any }]))
