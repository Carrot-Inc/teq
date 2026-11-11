// An export forwarder of a given is a stable path whose type is the singleton of what it forwards: dotty's
// `candidateKind` reads the reference widened (`ref.widen`, Implicits.scala:223-234), so the forwarder of a
// `Marker & A` given provides `A`. The import's index reads it as the class test does, widened, and the imported
// forwarder beats the outer `fallback`: scalac 3.8.4 prints 13.
trait Marker
class A(val value: Int) extends Marker
object Lib:
  given mixed: (Marker & A) = new A(13)
object Exp:
  export Lib.mixed
object Fallback:
  given fallback: A = new A(99)
import Fallback.fallback
object Main:
  def main(args: Array[String]): Unit =
    import Exp.mixed
    println(summon[A].value)
