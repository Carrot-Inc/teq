//> using platform js
// A plain inline call of a `Byte` widened by an ascription and then ascribed `Any`: the widened
// type stays on the call's node under the reference ascription, so the
// value is boxed as the widened class.
object M:
  inline def b: Byte = 1

def kind(x: Any): String = x match
  case _: Byte => "byte"
  case _: Short => "short"
  case _: Int => "int"
  case _: Double => "double"
  case _ => "other"

@main def main(): Unit =
  println(kind((M.b: Short): Any))
  println(kind((M.b: Int): Any))
  println(kind((M.b: Double): Any))
