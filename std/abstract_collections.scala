package scala.collection

// The abstract classes a library's anonymous collection extends (cats' `toIterable`), over the
// std's traits.
abstract class AbstractIterable[+A] extends Iterable[A]
abstract class AbstractIterator[+A] extends Iterator[A]

/** scala-library's `collection.Map`: what the immutable `Map` and the mutable `HashMap` share. */
trait Map[K, +V] extends Iterable[(K, V)]:
  def get(key: K): Option[V]
  def contains(key: K): Boolean = get(key).isDefined
  def isDefinedAt(key: K): Boolean = contains(key)
  def apply(key: K): V = get(key) match
    case Some(v) => v
    case None => throw new NoSuchElementException("key not found: " + key.toString)
  def getOrElse[V1 >: V](key: K, default: => V1): V1 = get(key) match
    case Some(v) => v
    case None => default

object Map:
  def empty[K, V]: scala.Map[K, V] = scala.Map.empty[K, V]
  def apply[K, V](entries: (K, V)*): scala.Map[K, V] = scala.Map.from(entries)
  def from[K, V](entries: IterableOnce[(K, V)]): scala.Map[K, V] = scala.Map.from(entries)

// scala-library's `collection.Set`, above the immutable `Set` and the mutable `HashSet`: what a
// library's signature says of a set it builds either way (izumi-reflect's `breakRefinement`
// gives a `mutable.LinkedHashSet` as one).
trait Set[A] extends Iterable[A]:
  def contains(elem: A): Boolean
  def apply(elem: A): Boolean = contains(elem)
  def subsetOf(that: Set[A]): Boolean = forall(that.contains)

/** scala-library's `BuildFrom`: builds a collection like `from` with elements of type `A`, which
  * `ZIO.foreach` and `Future.traverse` take to return the collection they were given.
  */
trait BuildFrom[-From, -A, +C]:
  def fromSpecific(from: From)(it: IterableOnce[A]): C
  def newBuilder(from: From): scala.collection.mutable.Builder[A, C]
  def toFactory(from: From): Factory[A, C] = BuildFrom.factoryOf(this, from)

object BuildFrom extends BuildFromLowPriority1:
  def factoryOf[From, A, C](bf: BuildFrom[From, A, C], from: From): Factory[A, C] =
    new Factory[A, C]:
      def fromSpecific(it: IterableOnce[A]): C = bf.fromSpecific(from)(it)
      def newBuilder: scala.collection.mutable.Builder[A, C] = bf.newBuilder(from)

  implicit def buildFromMapOps[K0, V0, K, V]: BuildFrom[scala.Map[K0, V0], (K, V), scala.Map[K, V]] =
    new BuildFrom[scala.Map[K0, V0], (K, V), scala.Map[K, V]]:
      def fromSpecific(from: scala.Map[K0, V0])(it: IterableOnce[(K, V)]): scala.Map[K, V] = scala.Map.from(it)
      def newBuilder(from: scala.Map[K0, V0]): scala.collection.mutable.Builder[(K, V), scala.Map[K, V]] = scala.Map.newBuilder[K, V]

  implicit def buildFromSortedMapOps[K0, V0, K: Ordering, V]: BuildFrom[immutable.SortedMap[K0, V0], (K, V), immutable.SortedMap[K, V]] =
    new BuildFrom[immutable.SortedMap[K0, V0], (K, V), immutable.SortedMap[K, V]]:
      def fromSpecific(from: immutable.SortedMap[K0, V0])(it: IterableOnce[(K, V)]): immutable.SortedMap[K, V] = immutable.SortedMap.from(it)
      def newBuilder(from: immutable.SortedMap[K0, V0]): scala.collection.mutable.Builder[(K, V), immutable.SortedMap[K, V]] = immutable.SortedMap.newBuilder[K, V]

  implicit val buildFromString: BuildFrom[String, Char, String] =
    new BuildFrom[String, Char, String]:
      def fromSpecific(from: String)(it: IterableOnce[Char]): String = newBuilder(from).addAll(it).result()
      def newBuilder(from: String): scala.collection.mutable.Builder[Char, String] =
        scala.collection.mutable.ArrayBuffer.empty[Char].mapResult(_.mkString)

  implicit def buildFromArray[A: scala.reflect.ClassTag]: BuildFrom[Array[?], A, Array[A]] =
    new BuildFrom[Array[?], A, Array[A]]:
      def fromSpecific(from: Array[?])(it: IterableOnce[A]): Array[A] = taggedArray(iterableToArray(it))
      def newBuilder(from: Array[?]): scala.collection.mutable.Builder[A, Array[A]] =
        scala.collection.mutable.ArrayBuffer.empty[A].mapResult(_.toArray)

trait BuildFromLowPriority1 extends BuildFromLowPriority2:
  implicit def buildFromSortedSetOps[A0, A: Ordering]: BuildFrom[immutable.SortedSet[A0], A, immutable.SortedSet[A]] =
    new BuildFrom[immutable.SortedSet[A0], A, immutable.SortedSet[A]]:
      def fromSpecific(from: immutable.SortedSet[A0])(it: IterableOnce[A]): immutable.SortedSet[A] = immutable.SortedSet.from(it)
      def newBuilder(from: immutable.SortedSet[A0]): scala.collection.mutable.Builder[A, immutable.SortedSet[A]] = immutable.SortedSet.newBuilder[A]

  implicit def fallbackStringCanBuildFrom[A]: BuildFrom[String, A, IndexedSeq[A]] =
    new BuildFrom[String, A, IndexedSeq[A]]:
      def fromSpecific(from: String)(it: IterableOnce[A]): IndexedSeq[A] = IndexedSeq.from(it)
      def newBuilder(from: String): scala.collection.mutable.Builder[A, IndexedSeq[A]] =
        scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => IndexedSeq.from(items))

trait BuildFromLowPriority2:
  // A collection builds its own kind, which conforms to the static `CC` it was seen as: a `List`
  // seen as a `Seq` builds a `List`, a `Range` seen as an `IndexedSeq` builds an `IndexedSeq`.
  implicit def buildFromIterableOps[CC[X] <: Iterable[X], A0, A]: BuildFrom[CC[A0], A, CC[A]] =
    new BuildFrom[CC[A0], A, CC[A]]:
      def fromSpecific(from: CC[A0])(it: IterableOnce[A]): CC[A] =
        unsafeCast[CC[A0], IterableOps[A0, CC, ?]](from).buildCC(iterableToArray(it))
      def newBuilder(from: CC[A0]): scala.collection.mutable.Builder[A, CC[A]] =
        scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => fromSpecific(from)(items))

  implicit def buildFromIterator[A]: BuildFrom[Iterator[?], A, Iterator[A]] =
    new BuildFrom[Iterator[?], A, Iterator[A]]:
      def fromSpecific(from: Iterator[?])(it: IterableOnce[A]): Iterator[A] = it.iterator
      def newBuilder(from: Iterator[?]): scala.collection.mutable.Builder[A, Iterator[A]] =
        scala.collection.mutable.ArrayBuffer.empty[A].mapResult(_.iterator)

/** The companion of a collection built with evidence of its element type: scala-library's
  * `scala.collection.EvidenceIterableFactory` (`SortedSet` over an `Ordering`).
  */
trait EvidenceIterableFactory[+CC[_], Ev[_]]:
  def from[A](it: IterableOnce[A])(implicit ev: Ev[A]): CC[A]
  def empty[A](implicit ev: Ev[A]): CC[A]
  def newBuilder[A](implicit ev: Ev[A]): scala.collection.mutable.Builder[A, CC[A]]

object EvidenceIterableFactory:
  implicit def toFactory[Ev[_], A, CC[_]](factory: EvidenceIterableFactory[CC, Ev])(implicit ev: Ev[A]): Factory[A, CC[A]] =
    new Factory[A, CC[A]]:
      def fromSpecific(it: IterableOnce[A]): CC[A] = factory.from(it)
      def newBuilder: scala.collection.mutable.Builder[A, CC[A]] = factory.newBuilder[A]

// scala-library's views by their names in this package, which the std's one `View` stands for.
type View[+A] = scala.View[A]
type SeqView[+A] = scala.View[A]
type IndexedSeqView[+A] = scala.View[A]
