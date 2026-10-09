// A class of the program extends the std's Iterator with `hasNext` and `next()`, or its Map with
// `get`, `iterator`, `updated` and `removed`; the other members follow from those, as they do
// for a class read from a jar.
class Counter(n: Int) extends Iterator[Int]:
  private var i = 0
  def hasNext: Boolean = i < n
  def next(): Int = { i += 1; i }
def iterators(): Unit =
  println(new Counter(3).toList)
  println(new Counter(5).map(_ * 2).filter(_ > 4).mkString(","))
  println(Iterator(1, 2, 3).zip(new Counter(9)).toList)
  println(List(1, 2, 3).iterator.grouped(2).toList)
  println((1 to 4).iterator.sliding(2).map(_.sum).toList)
  println(new Counter(4).span(_ < 3))
  println((new Counter(2) ++ new Counter(2) ++ Iterator(9)).toList)

class Wrapped[K, V](m: scala.collection.mutable.Map[K, V]) extends Map[K, V]:
  override def size: Int = m.size
  def get(k: K): Option[V] = m.get(k)
  override def iterator: Iterator[(K, V)] = m.iterator
  def updated[V2 >: V](key: K, value: V2): Map[K, V2] = m.toMap + ((key, value))
  def removed(key: K): Map[K, V] = m.toMap - key
def maps(): Unit =
  val w = new Wrapped(scala.collection.mutable.Map("a" -> 1, "b" -> 2))
  println(w.size)
  println(w.get("a"))
  println(w("b"))
  println(w.updated("c", 3).toList.sorted)
  println((w - "a").keys.toList)
  println(w == Map("a" -> 1, "b" -> 2))
  println(w.map((k, v) => (k, v * 10)))
  println(w.withDefaultValue(0)("zzz"))
  println(Map(1 -> "x").updated(2, "y").removed(1))
  println(Map(1 -> 2) ++ w.map((k, v) => (v, v)))

@main def run(): Unit =
  iterators()
  maps()
