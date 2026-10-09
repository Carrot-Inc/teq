package meridian.core.http

/** How two inputs combine: a unit disappears, a value joins a tuple, and the tuple grows. The
  * overlapping instances are ranked by the trait ladder, the more specific in the subclass. */
trait Concat[A, B, AB]:
  def split(ab: AB): (A, B)
  def join(a: A, b: B): AB

trait ConcatLow3:
  given pair2[A, B]: Concat[A, B, (A, B)] with
    def split(ab: (A, B)) = ab
    def join(a: A, b: B) = (a, b)

trait ConcatLow2 extends ConcatLow3:
  given pair3[A, B, C]: Concat[(A, B), C, (A, B, C)] with
    def split(ab: (A, B, C)) = ((ab._1, ab._2), ab._3)
    def join(a: (A, B), b: C) = (a._1, a._2, b)
  given pair4[A, B, C, D]: Concat[(A, B, C), D, (A, B, C, D)] with
    def split(ab: (A, B, C, D)) = ((ab._1, ab._2, ab._3), ab._4)
    def join(a: (A, B, C), b: D) = (a._1, a._2, a._3, b)
  given pair5[A, B, C, D, E]: Concat[(A, B, C, D), E, (A, B, C, D, E)] with
    def split(ab: (A, B, C, D, E)) = ((ab._1, ab._2, ab._3, ab._4), ab._5)
    def join(a: (A, B, C, D), b: E) = (a._1, a._2, a._3, a._4, b)
  given pair6[A, B, C, D, E, F]: Concat[(A, B, C, D, E), F, (A, B, C, D, E, F)] with
    def split(ab: (A, B, C, D, E, F)) = ((ab._1, ab._2, ab._3, ab._4, ab._5), ab._6)
    def join(a: (A, B, C, D, E), b: F) = (a._1, a._2, a._3, a._4, a._5, b)

trait ConcatLow1 extends ConcatLow2:
  given unitLeft[B]: Concat[Unit, B, B] with
    def split(ab: B) = ((), ab)
    def join(a: Unit, b: B) = b
  given unitRight[A]: Concat[A, Unit, A] with
    def split(ab: A) = (ab, ())
    def join(a: A, b: Unit) = a

object Concat extends ConcatLow1:
  given units: Concat[Unit, Unit, Unit] with
    def split(ab: Unit) = ((), ())
    def join(a: Unit, b: Unit) = ()
