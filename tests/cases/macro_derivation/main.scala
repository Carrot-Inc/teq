package app

import idlib.*

final case class LoanId(value: Long) derives LongKey

enum Color derives Enumerated:
  case Red, Green, Blue

enum Tone(val entryName: String) derives Labelled:
  case Loud extends Tone("loud")
  case Soft extends Tone("soft")

enum Unit_(val entryName: String) derives Enumerated:
  case Metre extends Unit_("m")
  case Second extends Unit_("s")

enum Shape derives Labelled:
  case Dot
  case Box(w: Int)

object Colors:
  def fromShort(s: String): Option[Color] = Enumerated[Color].valueOfIgnoreCase(s)

@main def run(): Unit =
  val id = LoanId(42L)
  println(LongKey[LoanId].keyName)
  println(id.value)
  println(LongKey[LoanId].wrap(7L))
  println(Enumerated[Color].enumName)
  println(Enumerated[Color].size)
  println(Enumerated[Color].valueList)
  println(Enumerated[Color].valueOf("Green"))
  println(Colors.fromShort("BLUE"))
  println(Enumerated[Color].fromOrdinal(5))
  println(Enumerated[Color].entryName(Color.Green))
  println(Color.Blue.ordinal)
  println(Enumerated[Unit_].valueList.map(_.entryName))
  println(Enumerated[Unit_].valueOf("s"))
  println(Labelled[Shape].enumName)
  println(Shape.Box(3).entryName)
  println(Shape.Dot.entryName)
  println(Labelled[Tone].enumName)
  println(Tone.Soft.entryName)
