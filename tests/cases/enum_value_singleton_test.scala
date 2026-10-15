// A type test against an enum value's singleton type is the test of that value (TypeTestsCasts,
// `x.isInstanceOf[p.type]` as `p eq x`), whichever form the type takes: written, in a pattern, in a
// `TypeTest` the compiler synthesizes, in an inline body; what a sum mirror's element types name.
enum Color:
  case Red, Green
inline def isOf[T](x: Any): Boolean = x.isInstanceOf[T]
@main def run(): Unit =
  val c: Any = Color.Green
  println(c.isInstanceOf[Color.Green.type])
  println(c.isInstanceOf[Color.Red.type])
  println(isOf[Color.Green.type](c))
  val pf: PartialFunction[Any, Any] = { case x: Color.Green.type => x }
  println(pf.isDefinedAt(c))
  println(summon[scala.reflect.TypeTest[Any, Color.Green.type]].unapply(c))
