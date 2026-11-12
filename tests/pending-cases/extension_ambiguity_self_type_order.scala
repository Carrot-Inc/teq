// A this type's `memberNames` are its `selfType`'s,
// `givenSelf & appliedRef` (Types.scala:1057-1058,
// 5597-5612), so `T.this`'s implicit members list the self
// type `S`'s `s` before T's own `t`, and scalac 3.8.4
// names s before t. Queued: push 6 (typeMismatch whole).
class R
trait Ops:
  extension (r: R) def pick: String
trait S:
  given s: Ops with
    extension (r: R) def pick: String = "s"
trait T:
  self: S =>
  given t: Ops with
    extension (r: R) def pick: String = "t"
  def run(): Unit = println((new R).pick)
object O extends T, S
object Main:
  def main(args: Array[String]): Unit = O.run()
