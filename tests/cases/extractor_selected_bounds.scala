trait Family {
  type Elem
  case class Pack[A <: Elem](value: A)
}
object Strings extends Family { type Elem = String }
def extractSelected(value: Any): Strings.Elem = value match {
  case Strings.Pack(item) => item
}
case class Invariant[A](value: A)
def extractInvariant(value: Any) = value match {
  case Invariant(item) => Invariant(item)
}
@main def extractorSelectedBounds(): Unit = {
  val widened: Invariant[?] = extractInvariant(Invariant(1))
  println(widened.value)
}
