// expect: 15:37: error: bad nested argument
// expect: 1 error found
// A transparent call's expansion inside an argument the extension's alternatives share holds a
// `compiletime.error` (an early expansion of a plain call): no cache keeps that typing, which a
// given-up alternative would keep without its error, and the chosen one reports it, as scalac.
import scala.compiletime.error
object M:
  inline def bad: Int = error("bad nested argument")
  transparent inline def wrap(inline x: Int): String = x.toString
trait Ops:
  extension [T](x: Int) def combine(y: String, z: Boolean): String = y
  extension [T](x: Int) def combine(y: String, z: Int): String = y
object O extends Ops
@main def main(): Unit =
  println(O.combine[Unit](1)(M.wrap(M.bad), 1))
