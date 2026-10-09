package units

// A value class of the program that the other file names in signatures only.
final class Meters(val value: Double) extends AnyVal:
  override def toString = s"${value}m"

object Meters:
  def of(d: Double): Meters = new Meters(d)
