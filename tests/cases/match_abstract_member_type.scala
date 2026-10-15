// A match on a value of an abstract type member reaches its `Boolean` and `String` cases
// (dotty's `provablyDisjoint` through the member's upper bound, Space.scala 747).
trait Property:
  type Out
  val valueOf: Out

object Render:
  def render(p: Property): String = p.valueOf match
    case x: Boolean => "flag " + x.toString
    case x: String => "text " + x
    case _ => "other"
  def literal(p: Property): String = p.valueOf match
    case true => "yes"
    case false => "no"
    case _ => "other"

def property[A](a: A): Property = new Property:
  type Out = A
  val valueOf = a

@main def main(): Unit =
  println(Render.render(property(true)))
  println(Render.render(property("x")))
  println(Render.render(property(3)))
  println(Render.literal(property(false)))
  println(Render.literal(property(true)))
  println(Render.literal(property('c')))
