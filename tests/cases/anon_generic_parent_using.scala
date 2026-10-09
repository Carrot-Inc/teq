// An anonymous class over a generic parent with a using clause: the parent's written type
// arguments bind the class's type, and the instance has the intersection of its parents.
trait Semi[A]:
  def combine(x: A, y: A): A
class MapMonoid[K, V](implicit V: Semi[V]):
  def empty: Map[K, V] = Map.empty
  def combine(x: Map[K, V], y: Map[K, V]): Map[K, V] =
    y.foldLeft(x) { case (acc, (k, v)) => acc.updated(k, acc.get(k).fold(v)(V.combine(_, v))) }
trait CommMonoid[A]:
  def tag: String = "comm"
given Semi[Int] with
  def combine(x: Int, y: Int): Int = x + y
def make[K, V](implicit ev: Semi[V]): MapMonoid[K, V] & CommMonoid[Map[K, V]] =
  new MapMonoid[K, V]()(using ev) with CommMonoid[Map[K, V]] {}
def make2[K, V](implicit ev: Semi[V]): MapMonoid[K, V] & CommMonoid[Map[K, V]] =
  new MapMonoid[K, V] with CommMonoid[Map[K, V]] {}
@main def run(): Unit =
  more()
  val m = make[String, Int]
  println(m.combine(Map("a" -> 1), Map("a" -> 2, "b" -> 3)))
  println(m.tag)
  println(make2[String, Int].combine(Map("x" -> 1), Map("x" -> 1)))
trait Named:
  def name: String
class Box[A](val a: A)(using n: Named):
  def label: String = s"${n.name}: $a"
given Named with
  def name: String = "box"
def boxed[A](a: A): Box[A] & Named = new Box[A](a) with Named { def name: String = "anon" }
def more(): Unit =
  println(boxed(7).label)
  println(boxed("s").name)
