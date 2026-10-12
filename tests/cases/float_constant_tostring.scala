// A `Float` or `Double` constant a type-level `ToString` reads is written as the JVM's
// `toString` writes it (`TypeEval` applies it): the shortest digits, in decimal from 10^-3 to
// below 10^7 and in scientific notation outside it, so `1f` is `"1.0"` and the largest `Float`
// `"3.4028235E38"`, which a literal type of that text accepts.
import scala.compiletime.constValue
import scala.compiletime.ops.any.ToString
@main def run() =
  println(constValue[ToString[3.4028235e38f]])
  println(constValue[ToString[1f]])
  println(constValue[ToString[1e7]])
  println(constValue[ToString[0.001]])
  println(constValue[ToString[1e-4]])
  println(constValue[ToString[123456.789f]])
  println(constValue[ToString[-0.0]])
  val one: "1.0" = constValue[ToString[1f]]
  println(one)
