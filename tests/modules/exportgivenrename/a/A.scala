package gra

trait Show[A]:
  def show(a: A): String

object A:
  given original: Show[Int] with
    def show(a: Int): String = "number " + a

object B:
  export A.{original as renamed}

// The givens of `B` are its export's, under the name `B` has it: `C`'s forwarder is `renamed`.
object C:
  export B.{given}
