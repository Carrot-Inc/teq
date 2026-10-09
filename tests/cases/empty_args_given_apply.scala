// `f()` for a parameterless `f` applies an `apply` extension a given provides for the result
// (scalac: "C 3").
class C(val n: Int)
trait Ops[T]:
  extension (t: T) def apply(): String
given Ops[C] with
  extension (t: C) def apply(): String = s"C ${t.n}"
def f: C = C(3)
@main def main(): Unit = println(f())
