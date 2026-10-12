// A variable a definition's inferred type fixes is instantiated on the side the constraint
// narrowed beyond its parameter's declared bounds (dotty's `Inferencing.instDirection`, ahead
// of the variance in `instDecision`): `mk((s: String) => ())` for `mk[A >: Null](f: A => Unit):
// Box[A]` is a `Box[String]`, its parameter's `Null` notwithstanding.
class Box[A](val name: String)
def mk[A >: Null](f: A => Unit): Box[A] = new Box[A]("box")
def mk2[A >: Null <: AnyRef, B <: A](x: B): Box[A] = new Box[A]("box2")
@main def run() =
  val x = mk((s: String) => ())
  val y: Box[String] = x
  val z = mk2("s")
  val w: Box[String] = z
  println(y.name)
  println(w.name)
