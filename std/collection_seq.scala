// scala-library's `collection.Seq` and `collection.IndexedSeq`, the traits above the std's `Seq` and `IndexedSeq`, in a
// file of their own: every program over sequences enters it.
package scala.collection

/** scala-library's `collection.Seq`: the sequences' common type, of which the std's `Seq` (scala-library's
  * `immutable.Seq`, the `Seq` package `scala` names) is one. A `collection.Seq` is no `Seq`: it takes no
  * immutable sequence's place and spreads into no repeated parameter. */
trait Seq[+A] extends SeqOps[A, Seq, Seq[A]], Iterable[A]:
  def equals(that: Any): Boolean = that match
    case other: Seq[?] => sameElements(other)
    case _ => false
  override def hashCode: Int = scala.seqHash(this)

object Seq:
  implicit def iterableFactory[A]: Factory[A, Seq[A]] = scala.Seq.iterableFactory[A]
  def apply[A](elems: A*): Seq[A] = elems.toList
  def empty[A]: Seq[A] = Nil
  def newBuilder[A]: mutable.Builder[A, Seq[A]] = List.newBuilder[A]
  def from[A](source: IterableOnce[A]): Seq[A] = List.from(source)
  def fill[A](n: Int)(elem: => A): Seq[A] = List.fill(n)(elem)
  def tabulate[A](n: Int)(f: Int => A): Seq[A] = List.tabulate(n)(f)
  def unapplySeq[A](x: Seq[A]): Some[Seq[A]] = Some(x)

/** scala-library's `collection.IndexedSeq`, of which the std's `IndexedSeq` (`immutable.IndexedSeq`) is one. */
trait IndexedSeq[+A] extends SeqOps[A, IndexedSeq, IndexedSeq[A]], Seq[A]

object IndexedSeq:
  def apply[A](elems: A*): IndexedSeq[A] = Vector.from(elems)
  def empty[A]: IndexedSeq[A] = Vector.empty[A]
  def from[A](source: IterableOnce[A]): IndexedSeq[A] = Vector.from(source)
  def newBuilder[A]: mutable.Builder[A, IndexedSeq[A]] = Vector.newBuilder[A]
