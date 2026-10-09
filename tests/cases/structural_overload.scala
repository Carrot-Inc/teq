// The second code pass's `structural_overload` program: the classes go to the `applyDynamic` alternative
// the call selects (dotty's `structuralCall`, `addClassOfs`), an overloaded one included.
class S extends Selectable:
  def applyDynamic(name: String, tags: Class[?]*)(args: Any*): Any = tags.length.toString + ":" + args.mkString(",")
  def applyDynamic(name: Int)(args: Any*): Any = "wrong overload"
def call(x: S { def f(a: Int): Any }): Any = x.f(7)
@main def run(): Unit = println(call(new S().asInstanceOf[S { def f(a: Int): Any }]))
