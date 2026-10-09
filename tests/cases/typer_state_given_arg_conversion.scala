// The same for a transparent given conversion: its argument's plain inline given `bad` fails in
// its own search, which takes `okInt`, and the conversion applies, as scalac and master print.
import scala.language.implicitConversions
class A
class B(val n: Int):
  def hello: String = s"B $n"
object A:
  transparent inline given conv(using inline x: Int): Conversion[A, B] = (a: A) => B(7)

trait LowInt:
  given okInt: Int = 5
object Ints extends LowInt:
  inline given bad: Int = scala.compiletime.error("bad failed")
import Ints.given

@main def run(): Unit = println(A().hello)
