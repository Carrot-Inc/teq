// A `val` constructor parameter that overrides a method (`toString`, `hashCode`) is a member
// called as that method on every target: string conversion, interpolation, `equals` through
// `hashCode`, and a call through a supertype.
final class Name(override val toString: String):
  def shout: String = toString.toUpperCase
class Coded(override val hashCode: Int, val label: String):
  override def equals(that: Any): Boolean = that match
    case c: Coded => c.hashCode == hashCode
    case _ => false
case class Tagged(override val toString: String, n: Int)
trait Shown:
  def toString: String
class ViaTrait(override val toString: String) extends Shown

@main def run(): Unit =
  val n = Name("ann")
  println(n)
  println(n.toString)
  println(s"$n!")
  println("" + n + "?")
  println(n.shout)
  println(List(n, Name("bob")))
  println(Some(n))
  val c = Coded(7, "seven")
  println(c.hashCode)
  println(c == Coded(7, "other"))
  println(Set(c, Coded(7, "x"), Coded(8, "y")).size)
  println(Tagged("t", 1))
  println(Tagged("t", 1).toString.length)
  val v: Shown = ViaTrait("via")
  println(v.toString)
  println(v)
  println((n: Any).toString)
