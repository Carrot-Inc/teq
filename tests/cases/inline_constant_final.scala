// An untyped `final val` of a literal is a constant whose singleton type an inline `if` reduces
// over.
object C:
  final val f = true
inline def choose(inline b: C.f.type): Int = inline if b then 1 else 0

@main def run(): Unit = println(choose(C.f))
