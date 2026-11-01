// The std collections' `knownSize` against scala-library's, empty and not. A collection that knows its size where
// scala-library's does not would give `Builder.sizeHint(coll, delta)` a hint scalac gives none for: the immutable
// sorted sets, `ListSet` and `ListMap` keep scala-library's -1 (an empty `ListSet` or `ListMap` 0). The std still
// reports -1 where scala-library's know their size, so a hint scalac gives is missing: `Nil`, `Vector`, `IndexedSeq`,
// `ArraySeq`, `Range`, an empty `LazyList`, an empty iterator and one of an array, a vector or a range,
// `ArrayBuffer`, `ListBuffer`, the mutable `Queue` and `Stack`, an array's and a string's sequence, a map's `values`.
// Queued: the std collections' `knownSize` as scala-library's.
import scala.collection.{immutable => im, mutable => mu}
@main def run(): Unit =
  def show(name: String, xs: IterableOnce[?]): Unit = println(s"$name ${xs.knownSize}")
  show("List", List(1, 2)); show("Nil", Nil); show("Vector", Vector(1, 2)); show("VectorE", Vector())
  show("Seq", Seq(1, 2)); show("IndexedSeq", IndexedSeq(1, 2)); show("ArraySeq", im.ArraySeq(1, 2))
  show("Range", 1 to 4); show("RangeE", 1 until 1); show("LazyList", LazyList(1, 2)); show("LazyListE", LazyList())
  show("Queue", im.Queue(1, 2)); show("QueueE", im.Queue())
  show("Set1", Set(1)); show("Set3", Set(1, 2, 3)); show("Set5", Set(1, 2, 3, 4, 5)); show("SetE", Set())
  show("HashSet", im.HashSet(1, 2)); show("ListSet", im.ListSet(1, 2)); show("ListSetE", im.ListSet())
  show("TreeSet", im.TreeSet(1, 2)); show("TreeSetE", im.TreeSet[Int]()); show("SortedSet", im.SortedSet(1, 2))
  show("Map1", Map(1 -> 1)); show("Map5", Map(1 -> 1, 2 -> 2, 3 -> 3, 4 -> 4, 5 -> 5)); show("MapE", Map())
  show("HashMap", im.HashMap(1 -> 1)); show("ListMap", im.ListMap(1 -> 1)); show("ListMapE", im.ListMap())
  show("TreeMap", im.TreeMap(1 -> 1)); show("TreeMapE", im.TreeMap[Int, Int]()); show("SortedMap", im.SortedMap(1 -> 1))
  show("Option", Some(1)); show("None", None)
  show("Iterator", Iterator(1, 2)); show("IteratorE", Iterator()); show("ListIter", List(1, 2).iterator)
  show("VecIter", Vector(1, 2).iterator); show("RangeIter", (1 to 3).iterator)
  show("ArrayBuffer", mu.ArrayBuffer(1, 2))
  show("ListBuffer", mu.ListBuffer(1, 2)); show("ListBufferE", mu.ListBuffer())
  show("mHashSet", mu.HashSet(1)); show("mHashMap", mu.HashMap(1 -> 1)); show("mTreeSet", mu.TreeSet(1, 2))
  show("mQueue", mu.Queue(1, 2)); show("mStack", mu.Stack(1, 2)); show("PQ", mu.PriorityQueue(1, 2))
  show("mLinkedHashSet", mu.LinkedHashSet(1)); show("mLinkedHashMap", mu.LinkedHashMap(1 -> 1))
  show("ArrayOps", Array(1, 2).toSeq); show("String", "ab".toSeq); show("View", List(1, 2).view)
  show("VecView", Vector(1, 2).view)
  show("VecViewMap", Vector(1, 2).view.map(_ + 1)); show("SeqMap", List(1).map(_ + 1))
  show("Keys", Map(1 -> 1).keySet); show("Values", Map(1 -> 1).values); show("Zip", List(1).zip(List(2)))
  show("Stream", Stream(1, 2)); show("Tuple", (1, 2).productIterator)
