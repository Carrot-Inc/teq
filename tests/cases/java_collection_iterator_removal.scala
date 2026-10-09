// Removal as the JDK's collections do it. `Collection.removeIf` and `AbstractCollection`'s
// `removeAll`, `retainAll`, `remove` and `clear` remove through the iterator as they meet each
// element, so a test sees the size shrink and an exception leaves what was not reached; a
// program's collection with a removable iterator takes part. `ArrayList` and `ArrayDeque` test every
// element first and remove after (`removeIf`, `batchRemove`, `bulkRemove`). A `HashMap`'s views
// remove from the map.
import java.util.*

class Bag extends AbstractCollection[String]:
  val data = new ArrayList[String]()
  data.add("a"); data.add("b"); data.add("c")
  def size(): Int = data.size()
  def iterator(): Iterator[String] = new Iterator[String]:
    var i = 0
    def hasNext(): Boolean = i < data.size()
    def next(): String = { val x = data.get(i); i += 1; x }
    override def remove(): Unit = { i -= 1; data.remove(i); () }

@main def run(): Unit =
  val b = new Bag
  println(b.removeIf(x => { println(x + ":" + b.size()); true }))
  println(b.data)
  val c = new Bag
  val remove = new ArrayList[String](); remove.add("b")
  println(c.removeAll(remove))
  println(c.data)
  val d = new Bag
  println(s"${d.remove("c")} ${d.data}")
  d.clear()
  println(d.data)
  val e = new Bag
  try e.removeIf(x => { if x == "b" then throw new IllegalStateException(); true })
  catch case _: IllegalStateException => println("stopped")
  println(e.data)

  val list = new ArrayList[String](); list.add("a"); list.add("a"); list.add("b")
  var seen = 0
  println(list.removeIf(x => { println(x + ":" + list.size()); seen += 1; seen == 2 }))
  println(list)
  val it = list.iterator(); it.next(); it.remove()
  println(list)
  try it.remove() catch case _: IllegalStateException => println("twice")

  val set = new HashSet[Int](); set.add(1); set.add(2); set.add(3)
  println(set.removeIf(x => { println(x + ":" + set.size()); x != 2 }))
  println(set)
  val keep = new HashSet[Int](); keep.add(2); keep.add(9)
  set.add(5)
  println(s"${set.retainAll(keep)} $set")

  val map = new HashMap[String, Int](); map.put("x", 1); map.put("y", 2); map.put("z", 3)
  println(s"${map.keySet().removeIf(_ == "x")} $map")
  println(s"${map.values().removeIf(_ == 2)} $map")
  map.put("w", 4)
  println(s"${map.entrySet().removeIf(_.getValue == 4)} $map")
  val ks = map.keySet()
  map.put("v", 5)
  println(s"${ks.size()} ${ks.contains("v")}")
  ks.remove("v")
  println(map)
  map.values().clear()
  println(map.isEmpty())

  val deque = new ArrayDeque[Int](); deque.add(1); deque.add(2); deque.add(3)
  println(deque.removeIf(x => { println(x + ":" + deque.size()); x % 2 == 1 }))
  println(deque)

  val tree = new TreeSet[Int](); tree.add(3); tree.add(1); tree.add(2)
  println(s"${tree.removeIf(x => { println(x + ":" + tree.size()); x < 3 })} $tree")
