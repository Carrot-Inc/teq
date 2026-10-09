//> using platform js
// A plain inline call of a `Byte`, `Short` or `Float` ascribed `Any`: the later expansion keeps
// the call's own type under the ascription's, so the value is boxed as
// its own class.
object M:
  inline def b: Byte = 1
  inline def s: Short = 2
  inline def f: Float = 3.0f

def kind(x: Any): String = x match
  case _: Byte => "byte"
  case _: Short => "short"
  case _: Float => "float"
  case _: Int => "int"
  case _: Double => "double"
  case _ => "other"

@main def main(): Unit =
  println(kind(M.b: Any))
  println(kind(M.s: Any))
  println(kind(M.f: Any))
