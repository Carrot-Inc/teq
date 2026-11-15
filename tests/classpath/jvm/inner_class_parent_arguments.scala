// jars: scala-library
// std: scala-library
// targets: jvm
// A class of a library nested in a class is constructed under the descriptor its class file declares, its enclosing
// instance first: extended with arguments (`Enumeration#Val(i: Int, name: String)` inside an `Enumeration`), extended
// without (the secondary constructor), and as an anonymous class through the primary and through a secondary
// constructor, which gives the anonymous class its own parameter list, its parent call and its creation.
object Levels extends Enumeration:
  class Level(i: Int, name: String, val weight: Int) extends Val(i, name)
  val Low = new Level(0, "low", 1)
  val High = new Level(1, "high", 9)

object Planets extends Enumeration:
  class PlanetVal(val mass: Double) extends Val
  val Earth = new PlanetVal(5.97)
  val Mars = new PlanetVal(0.64)

object Marks extends Enumeration:
  val Star = new Val("star") { override def toString: String = "*" + super.toString }
  val Dash = new Val(7, "dash") { override def toString: String = "-" + super.toString }
  val Bare = new Val { override def toString: String = "bare" + id }

// The path of the enclosing instance may be one the verifier does not take for the constructor's parameter (a
// member of a generic value is an `Object`): it is cast, through the secondary constructor and through the primary.
class Holder[T](val v: T)
object Tags extends Enumeration:
  val self: Holder[Tags.type] = new Holder(this)
  class Plain extends self.v.Val
  class Named(i: Int, name: String) extends self.v.Val(i, name)
  val One = new Plain
  val Two = new Named(5, "two")

@main def main(): Unit =
  println(Levels.High.toString + Levels.High.weight + Levels.Low.id)
  println(Planets.Earth.mass + Planets.Mars.mass)
  println(Planets.values.size)
  println(Marks.Star.toString + Marks.Dash + Marks.Bare + Marks.Dash.id)
  println(Tags.One.id.toString + Tags.Two + Tags.Two.id)
