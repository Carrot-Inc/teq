package tpa

// Trait parameters a class over the products passes as a class of the same build does (dotty's
// `Mixin`, `traitInits`): a val's accessor, a private parameter's expanded one (`tpa$Counted$$start`),
// a var's setter, a using parameter's (`tpa$Sorted$$ord`), a default's getter on the trait's
// companion, each set before the trait's `$init$`. scalac's downstream implements the same.
trait Counted(val label: String, start: Int, var count: Int = 0):
  println("Counted " + label + " " + start)
  val doubled: Int = start * 2
  def bump(): Int = { count += 1; count + start }

trait Sorted[A](using ord: Ordering[A]):
  def sort(xs: List[A]): List[A] = xs.sorted(using ord)

trait Named(val first: String, val last: String = "Doe"):
  def full: String = first + " " + last
