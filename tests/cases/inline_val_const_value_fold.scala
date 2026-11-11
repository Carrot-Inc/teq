// An inline val's right-hand side folds over a transparent call's result: an expansion in an `Inlined`, and
// `constValue`'s, which is the constant alone (`InlineCall.expand`'s `constValueOrError`). scalac prints the lines
// of the .expected file.
import scala.compiletime.constValue
transparent inline def three(): Int = 3
transparent inline def threeOf[T]: Int = 3
transparent inline def valueOf[T]: T = constValue[T]
object O:
  inline val a = three() * 2
  inline val b = threeOf[Int] * 2
  inline val c = valueOf[4] * 2
  inline val d = constValue[3] * 2
@main def run(): Unit = println((O.a, O.b, O.c, O.d))
