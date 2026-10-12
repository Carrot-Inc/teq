// The decimal `Float.toString` and `Double.toString` select (`TypeEval`'s `ToString`): of the
// decimals that round to the value, the fewest digits, but two where one would do and two are
// closer, so `Float.MIN_VALUE` is `1.4E-45` (one digit gives `1.0E-45`) and `Double.MIN_VALUE`
// `4.9E-324`; a literal type of that text accepts it.
import scala.compiletime.constValue
import scala.compiletime.ops.any.ToString
@main def run() =
  val f: "1.4E-45" = constValue[ToString[1.4e-45f]]
  val d: "4.9E-324" = constValue[ToString[4.9e-324]]
  println(f)
  println(d)
  println(constValue[ToString[2.8e-45f]])
  println(constValue[ToString[4.2e-45f]])
  println(constValue[ToString[9.9e-324]])
  println(constValue[ToString[1.5e-323]])
  println(constValue[ToString[1e-3f]])
  println(constValue[ToString[0.002]])
  println(constValue[ToString[1.17549435E-38f]])
  println(constValue[ToString[1.0E-5f]])
  println(constValue[ToString[1e23]])
  println(constValue[ToString[2e22]])
  println(constValue[ToString[100f]])
  println(constValue[ToString[7e6]])
  println(constValue[ToString[3e7f]])
