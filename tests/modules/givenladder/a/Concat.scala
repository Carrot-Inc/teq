package gla

// A companion's givens inherited from the trait ladder that ranks them, the more specific in the
// subclass: in the implicit scope of `Concat[..]` whether the companion is a source or read from
// products.
trait Concat[A, B, AB]:
  def join(a: A, b: B): AB

trait ConcatLow:
  given pair2[A, B]: Concat[A, B, (A, B)] with
    def join(a: A, b: B) = (a, b)

object Concat extends ConcatLow:
  given units: Concat[Unit, Unit, Unit] with
    def join(a: Unit, b: Unit) = ()
