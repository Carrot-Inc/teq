// scala-library's converters between the Scala and the Java collections, by the names compiled
// library bodies call (`CollectionConverters.IterableHasAsJava(xs).asJavaCollection`): the Java
// side is a view of the Scala collection, the Scala side a copy of the Java one.
package scala.jdk:

  object CollectionConverters:
    implicit class IterableHasAsJava[A](i: Iterable[A]):
      def asJava: java.lang.Iterable[A] = new scala.collection.convert.IterableAsJava(i)
      def asJavaCollection: java.util.Collection[A] = new scala.collection.convert.IterableAsJava(i)
    implicit class IteratorHasAsJava[A](i: Iterator[A]):
      def asJava: java.util.Iterator[A] = new scala.collection.convert.IteratorAsJava(i)
    implicit class SeqHasAsJava[A](s: Seq[A]):
      def asJava: java.util.List[A] = new scala.collection.convert.SeqAsJava(s)
    implicit class ListHasAsScala[A](l: java.util.List[A]):
      def asScala: scala.collection.mutable.ArrayBuffer[A] = new scala.collection.mutable.ArrayBuffer(buffered(l.toArray(untaggedArray(emptyArray[A]))))
    implicit class CollectionHasAsScala[A](c: java.util.Collection[A]):
      def asScala: Iterable[A] = new scala.collection.mutable.ArrayBuffer(buffered(c.toArray(untaggedArray(emptyArray[A]))))
    implicit class IteratorHasAsScala[A](i: java.util.Iterator[A]):
      def asScala: Iterator[A] = new scala.collection.convert.IteratorAsScala(i)

package scala.collection:

  object JavaConverters:
    implicit def asJavaCollectionConverter[A](i: Iterable[A]): AsJavaCollection[A] = new AsJavaCollection(i)
    implicit def seqAsJavaListConverter[A](s: Seq[A]): AsJava[java.util.List[A]] = new AsJava(new scala.collection.convert.SeqAsJava(s))
    implicit def asScalaIteratorConverter[A](i: java.util.Iterator[A]): AsScala[Iterator[A]] = new AsScala(new scala.collection.convert.IteratorAsScala(i))
    implicit def asJavaIteratorConverter[A](i: Iterator[A]): AsJava[java.util.Iterator[A]] = new AsJava(new scala.collection.convert.IteratorAsJava(i))
    implicit def asScalaBufferConverter[A](l: java.util.List[A]): AsScala[scala.collection.mutable.Buffer[A]] =
      new AsScala(scala.collection.convert.bufferOver(l))
    final class AsJavaCollection[A](i: Iterable[A]):
      def asJavaCollection: java.util.Collection[A] = new scala.collection.convert.IterableAsJava(i)
    final class AsJava[A](op: => A):
      def asJava: A = op
    final class AsScala[A](op: => A):
      def asScala: A = op

package scala.collection.convert:

  // A Java list as a Scala buffer: over an `ArrayList`, the list's own storage, so that an
  // append or a removal on either side is seen on the other, as scala-library's wrapper sees
  // them; any other list is copied.
  def bufferOver[A](l: java.util.List[A]): scala.collection.mutable.ArrayBuffer[A] = l match
    case a: java.util.ArrayList[A] => new scala.collection.mutable.ArrayBuffer(listStorage(a))
    case _ => scala.collection.mutable.ArrayBuffer.from(new IteratorAsScala(l.iterator()))

  @jvm("$0")
  def listStorage[A](list: java.util.ArrayList[A]): RawBuffer[A] = buffered(java.util.arrayListStorage(list))

  final class IterableAsJava[A](underlying: Iterable[A]) extends java.util.Collection[A]:
    def size(): Int = underlying.size
    override def isEmpty(): Boolean = underlying.isEmpty
    def iterator(): java.util.Iterator[A] = new IteratorAsJava(underlying.iterator)

  final class SeqAsJava[A](underlying: Seq[A]) extends java.util.List[A]:
    def size(): Int = underlying.length
    def get(index: Int): A =
      if index < 0 || index >= underlying.length then throw new IndexOutOfBoundsException(index.toString)
      underlying(index)

  final class IteratorAsJava[A](underlying: Iterator[A]) extends java.util.Iterator[A]:
    def hasNext: Boolean = underlying.hasNext
    def next(): A = underlying.next()

  final class IteratorAsScala[A](underlying: java.util.Iterator[A]) extends scala.collection.AbstractIterator[A]:
    def hasNext: Boolean = underlying.hasNext
    def next(): A = underlying.next()
