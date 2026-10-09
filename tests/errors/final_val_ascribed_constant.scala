// A `final val` whose initialiser is ascribed a type that is no literal type (`(0: Byte)`) has
// that type, no constant type, as scalac types the ascription: no constant to an `inline if`,
// where `final val plain = 0` is one.
// expect: 11:21: error: Cannot reduce `inline if` because its condition is not a constant value: Consts.b == 0
object Consts:
  final val b = (0: Byte)
  final val plain = 0
inline def g: String = inline if Consts.b == 0 then "constant" else "no"
inline def h: String = inline if Consts.plain == 0 then "constant" else "no"
@main def run(): Unit =
  println(h + " " + g)
