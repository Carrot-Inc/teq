// The second code pass's `structural_value_class` program.
class Meter(val n: Int) extends AnyVal
class S extends Selectable:
  def applyDynamic(name: String, tags: Class[?]*)(args: Any*): Any = tags.map(_.getName()).mkString(",")
def call(x: S { def f(a: Meter): Any }): Any = x.f(new Meter(3))
@main def run(): Unit = println(call(new S().asInstanceOf[S { def f(a: Meter): Any }]))
