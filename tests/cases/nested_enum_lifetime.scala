// An enum nested in a class or a trait has a companion per enclosing instance, as an object nested
// there has: its values are made with that instance (eagerly where the companion is the one the
// enum makes in a class, lazily where the source writes it or the enum is a trait's) and read
// through it, and its members read the instance.
class Box(val n: Int):
  enum Kind:
    case A, B
    case C(x: Int)
    println(s"case of $n")
    def get: Int = this match
      case A => n
      case B => n + 1
      case C(x) => n + x
  def first: Kind = Kind.A

class Lazy:
  enum Mode:
    case On
    println("mode case")
  object Mode:
    def default: Mode = On

trait Holder:
  def tag: String
  enum Shade:
    case Light, Dark
    def show: String = s"$tag-$this"

class Lamp(val tag: String) extends Holder

class Plain:
  object Inner

@main def run(): Unit =
  val a = new Box(1)
  val b = new Box(10)
  println("boxes made")
  println(a.Kind.A eq a.Kind.A)
  println(a.Kind.A eq b.Kind.A)
  println(a.first eq a.Kind.A)
  println(s"${a.Kind.A.get} ${b.Kind.B.get} ${a.Kind.C(5).get} ${b.Kind.C(5).get}")
  println(s"${a.Kind.B.ordinal} ${b.Kind.C(2)} ${a.Kind.C(2) == a.Kind.C(2)}")
  val l = new Lazy
  println("lazy made")
  println(l.Mode.default eq l.Mode.On)
  println(l.Mode.On eq (new Lazy).Mode.On)
  println(s"${l.Mode.values.toList} ${l.Mode.valueOf("On") eq l.Mode.fromOrdinal(0)}")
  val x = new Lamp("x")
  val y = new Lamp("y")
  println(s"${x.Shade.Dark.show} ${y.Shade.Light.show}")
  println(x.Shade.Light eq x.Shade.Light)
  println(x.Shade.Light eq y.Shade.Light)
  println(x.Shade.values.map(_.show).mkString(","))
  val p = new Plain
  println(p.Inner eq p.Inner)
  println(p.Inner eq (new Plain).Inner)
