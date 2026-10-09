package ega

trait Show[A]:
  def show(a: A): String

object A:
  given intShow: Show[Int] with
    def show(a: Int): String = "number=" + a

// An export of the givens alone: its selector is the `given` one, which a downstream reads from
// the pickle as the empty name.
object B:
  export A.{given}
