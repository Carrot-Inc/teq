// The classes of a structural call's parameters go to the `applyDynamic` the call selects, as
// dotty's `structuralCall` types the call and `addClassOfs` then reads the selected method's
// instantiated type: an overloaded one whose `String` alternative takes `Class[?]*`, one whose
// selected alternative takes none, and a generic one instantiated at `Class[?]`.
class Both extends Selectable:
  def applyDynamic(name: String, tags: Class[?]*)(args: Any*): Any = "classes " + tags.length + ":" + args.mkString(",")
  def applyDynamic(name: Int)(args: Any*): Any = "wrong overload"

class Plain extends Selectable:
  def applyDynamic(name: String)(args: Any*): Any = "plain:" + args.mkString(",")
  def applyDynamic(name: Int, tags: Class[?]*)(args: Any*): Any = "wrong overload"

class Generic[C] extends Selectable:
  def applyDynamic(name: String, tags: C*)(args: Any*): Any = "generic " + tags.length + ":" + args.mkString(",")

def both(x: Both { def f(a: Int, b: String): Any }): Any = x.f(7, "s")
def plain(x: Plain { def f(a: Int): Any }): Any = x.f(8)
def generic(x: Generic[Class[?]] { def f(a: Int): Any }): Any = x.f(9)
def strings(x: Generic[String] { def f(a: Int): Any }): Any = x.f(10)

@main def run(): Unit =
  println(both(new Both().asInstanceOf[Both { def f(a: Int, b: String): Any }]))
  println(plain(new Plain().asInstanceOf[Plain { def f(a: Int): Any }]))
  println(generic(new Generic[Class[?]]().asInstanceOf[Generic[Class[?]] { def f(a: Int): Any }]))
  println(strings(new Generic[String]().asInstanceOf[Generic[String] { def f(a: Int): Any }]))
