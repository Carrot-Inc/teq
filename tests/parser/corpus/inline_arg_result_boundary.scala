// An inline call's result has the type the call has at the definition, whatever parameter
// the body returns as written: `pick(id(x))` ranks at `id`'s `Any`, not at the parameter's `B`.
trait B
object O extends B
def pick(x: Any): String = "any"
def pick(x: B): String = "base"
def pick(x: O.type): String = "object"
inline def id(x: Any): Any = x
inline def same(x: B): B = x
inline def f(x: B): String = pick(id(x)) + " " + pick(same(x)) + " " + pick((id(x): Any))
@main def run(): Unit = println(f(O))
