// extension_ambiguity_self_type_order with `S` declared after `T`: the
// self type's names still come first (`selfType`, `givenSelf & appliedRef`,
// Types.scala:1057-1058, 5597-5612), whatever the symbols' age, and
// scalac 3.8.4 names s before t. Queued: push 6 (typeMismatch whole).
class R
trait Ops:
  extension (r: R) def pick: String
trait T:
  self: S =>
  given t: Ops with
    extension (r: R) def pick: String = "t"
  def run(): Unit = println((new R).pick)
trait S:
  given s: Ops with
    extension (r: R) def pick: String = "s"
object O extends T, S
object Main:
  def main(args: Array[String]): Unit = O.run()
