// The companion the compiler makes for a case class nested in a class or a trait, and for a class with constructor
// defaults, read before and after its class's definition in the body. scalac makes a class's own made companion
// eagerly (dotty's transformSyntheticModule, `public final C$lzyN` set first in the constructor) where
// allOverriddenSymbols finds nothing, and a trait's through Mixin's forwarder as a thread-safe lazy val; teq holds
// every made companion in a thread-safe lazy val until the typer's allOverriddenSymbols lands (a queued item), so the
// values are the same and the class files differ. Also a made companion beside a parent's member of its name that it
// does not override (an overload taking a parameter), and dotty's t704 (tests/cases/scala3_t704).
trait Shapes:
  case class Square(side: Int)
  def unit: Square = Square(1)

class Plane extends Shapes:
  val early: Circle = Circle(2)
  case class Circle(r: Int)
  class Rect(val w: Int = 3, val h: Int = 4)
  def area: Int = { val r = new Rect(); r.w * r.h }

class Base:
  def Dot(name: String): String = "Base.Dot(" + name + ")"

class Derived extends Base:
  case class Dot(i: Int)

object Main:
  def main(args: Array[String]): Unit =
    val p = new Plane
    println(p.early)
    println(p.Circle(5))
    println(p.area)
    println(p.unit)
    println(p.Square(7))
    val d = new Derived
    println(new d.Dot(3))
    println(d.Dot("named"))
