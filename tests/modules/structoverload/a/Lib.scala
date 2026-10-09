package sol

// A structural call through a product: the classes go to the overloaded `applyDynamic` the call
// selects (dotty's `structuralCall`, `addClassOfs`; the second code pass's `structural_overload`).
class S extends Selectable:
  def applyDynamic(name: String, tags: Class[?]*)(args: Any*): Any = tags.length.toString + ":" + args.mkString(",")
  def applyDynamic(name: Int)(args: Any*): Any = "wrong overload"

def call(x: S { def f(a: Int): Any }): Any = x.f(7)
def exercise(): Unit = println(call(new S().asInstanceOf[S { def f(a: Int): Any }]))
