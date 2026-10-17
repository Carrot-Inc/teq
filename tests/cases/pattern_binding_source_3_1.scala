// teq: --source 3.1 --werror
//> using options -source:3.1
// Before 3.2 no pattern definition is checked (`Checking.checkIrrefutable`'s `recur`, true where
// `!sourceVersion.isAtLeast(3.2)`): a narrowing and a refutable extractor pass silently.
package patternbindingsource31

object Ex:
  def unapply(s: String): Option[Int] = if s.nonEmpty then Some(s.length) else None

@main def main(): Unit =
  val h :: t = List(1, 2)
  val Some(o) = Option(3)
  val Ex(e) = "four"
  println(s"$h $t $o $e")
