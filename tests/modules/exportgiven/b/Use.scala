package egb

import ega.*

enum Color derives Named:
  case Red, Green

object ByName:
  import ega.Utils.{*, given}
  def show(c: Color): String = summon[Enc[Color]].enc(c)
  def all(cs: Set[Color]): String = summon[Enc[Set[Color]]].enc(cs)

object BySelector:
  import ega.Prelude.{*, given}
  def show(c: Color): String = summon[Enc[Color]].enc(c)

object Wildcard:
  import ega.Everything.*
  def run: String = area(Box(2)) + " " + area(Box(2, 3), unit = Defaults.unit)

@main def run(): Unit =
  println(ByName.show(Color.Red))
  println(ByName.all(Set(Color.Green)))
  println(BySelector.show(Color.Green))
  println(Wildcard.run)
