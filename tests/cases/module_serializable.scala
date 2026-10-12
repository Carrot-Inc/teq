//> using platform js
// Every module class is `java.io.Serializable` on the JVM (dotty's `BCodeSkelBuilder`): an object
// of a package or of a class, an object nested in a local class, an enum's companion; not an
// object a block owns, whose class `LambdaLift` lifts without `Module`. Scala.js leaves objects as
// they are written.
object Top:
  def v = 1

class Holder:
  object Inner
  enum Color:
    case Red, Green

enum Plain:
  case A, B
object Plain:
  def first = A

def local(): Seq[Any] =
  object Loc
  class C:
    object Nested
  enum E:
    case X
  Seq(Loc, (new C).Nested, E)

@main def run(): Unit =
  val h = new Holder
  println(Seq[Any](Top, h.Inner, h.Color, Plain).map(_.isInstanceOf[java.io.Serializable]).mkString(" "))
  println(local().map(_.isInstanceOf[java.io.Serializable]).mkString(" "))
