package demandedtargs

final class Box[A](val value: A):
  def both = List(value, value)
  def shown = new Shown[A] { def show(a: A): String = "<" + a + ">" }
  inline def twice: Int = both.size
  inline def label: String = shown.show(value)

trait Shown[T]:
  def show(t: T): String
