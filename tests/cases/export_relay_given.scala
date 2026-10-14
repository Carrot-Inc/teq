// A given relayed through two objects' exports is one candidate: the second object's table leaves
// the first object's forwarder to its own forwarder (Namer.exportForwarders).
trait Eqv[A]:
  def eqv(x: A, y: A): Boolean
trait Named[E]:
  def name(e: E): String
object Codecs:
  given namedEqv: [T: Named] => Eqv[T] = (x: T, y: T) => summon[Named[T]].name(x) == summon[Named[T]].name(y)
object Utils:
  export Codecs.namedEqv
object Prelude:
  export Utils.given

enum Color:
  case Red, Green
object Color:
  given Named[Color] = (c: Color) => c.toString
object Use:
  import Prelude.{*, given}
  def run: Boolean = summon[Eqv[Color]].eqv(Color.Red, Color.Green)
@main def m(): Unit = println(Use.run)
