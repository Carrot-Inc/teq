// A kept body that constructs a class whose parent declares a private member of the same name (LHash and LEq, both
// with an `ev`): the products build names LEq's field as the whole build does, which sees LHash beside it (the
// emitter's private-name clash), so the two builds' JavaScript is the same bytes (cats-kernel's ListEq and ListHash).
package lib
trait Eqv[A]:
  def eqv(x: A, y: A): Boolean
trait Hsh[A] extends Eqv[A]:
  def hash(x: A): Int
class LEq[A](implicit ev: Eqv[A]) extends Eqv[List[A]]:
  def eqv(xs: List[A], ys: List[A]): Boolean = xs.length == ys.length && xs.zip(ys).forall((x, y) => ev.eqv(x, y))
class LHash[A](implicit ev: Hsh[A]) extends LEq[A] with Hsh[List[A]]:
  def hash(xs: List[A]): Int = xs.map(ev.hash).sum
object Inst:
  def eqL[A](implicit ev: Eqv[A]): Eqv[List[A]] = new LEq[A]
  def hashL[A](implicit ev: Hsh[A]): Hsh[List[A]] = new LHash[A]
