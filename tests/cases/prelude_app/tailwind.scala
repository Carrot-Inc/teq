package sharedlib.tailwind

final case class Tw(value: String):
  def isEmpty: Boolean = value.isEmpty
  def ++(other: Tw): Tw =
    if value.isEmpty then other
    else if other.value.isEmpty then this
    else Tw(value + " " + other.value)

object Tw:
  val empty: Tw = Tw("")

object TailwindSyntax:
  extension (sc: StringContext)
    def tw(args: Any*): Tw = Tw(sc.s(args*).trim)
