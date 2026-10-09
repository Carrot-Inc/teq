// `constValue` of a parameter's singleton is kept at the definition and taken at the expansion,
// where the argument is the constant `3`, as scalac 3.8.4 has it: the parameter's path is the
// literal's type in the copy. The expansion by substitution gives scalac's; the
// retype path reports `(x : Int) is not a constant type`.
import scala.compiletime.constValue
inline def value(x: Int): x.type = constValue[x.type]
@main def run(): Unit =
  val r: 3 = value(3)
  println(r)
