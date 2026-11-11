sealed trait Parcel[A]
case class TextParcel(value: String) extends Parcel[String]
def ordinary[A](parcel: Parcel[A]): A = parcel match
  case TextParcel(value) => value
// Demanding the inline definition must restore this branch's GADT evidence.
def enclosing[A](parcel: Parcel[A]): A = parcel match
  case TextParcel(value) =>
    val ignored = expanded(TextParcel("nested"))
    value
inline def expanded[A](parcel: Parcel[A]): A = inline parcel match
  case TextParcel(value) => value
@main def inlineDefinitionGadtScope(): Unit =
  println(ordinary(TextParcel("ordinary")))
  println(expanded(TextParcel("expanded")))
  println(enclosing(TextParcel("enclosing")))
