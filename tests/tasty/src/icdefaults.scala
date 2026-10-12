package fix.icdef

// Implicit classes whose constructors have a default: the companion scalac makes to hold the
// default's getter shares its term name with the class's conversion, and a body's `new Applied[R]`
// reads the default on that companion (zio's `ZLayer.ScopedPartiallyApplied`).
object IcDefaults:
  implicit final class Applied[R](private val dummy: Boolean = true) extends AnyVal:
    def apply(x: Int): Int = if dummy then x + 1 else x
  def applied[R]: Applied[R] = new Applied[R]
  implicit class Counted(val n: Int = 3):
    def twice: Int = n * 2
  def counted: Counted = new Counted
