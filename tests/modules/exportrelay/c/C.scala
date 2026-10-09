package erc

import era.*

object Prelude extends erb.PreludeCore

enum Color:
  case Red, Green

object Color:
  given Named[Color] = (c: Color) => c.toString

object Use:
  import Prelude.{*, given}
  import erz.*
  def run: String = describe(Color.Red) + " " + summon[Eqv[Color]].eqv(Color.Red, Color.Green)
  def boxed: Box[Int] = List(1, 2)
  def labelled(l: Label, c: Count): String = s"$l=$c"

@main def run(): Unit =
  println(Use.run)
  println(Use.boxed)
  println(Use.labelled("n", 3))
