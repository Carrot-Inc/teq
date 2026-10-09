import scala.quoted.*
import scala.collection.mutable

// Identity hashes of every kind of value the interpreter keeps one for, asked by the runs of
// several files: an object with a cycle, an array longer than the walk's bound of elements, two
// maps filled in two orders, an immutable map's trie, closures, an enum value and the objects a
// run makes. Each is a function of the value's contents, the
// same whichever worker's heap asks first.
class Node(var value: Int, var next: Node)
class Holder(val name: String, val items: Array[Int], val f: Int => Int)
enum Color:
  case Red, Green

object Values:
  val cyclic: Node =
    val n = new Node(1, null)
    n.next = n
    n
  val big: Array[Int] = Array.tabulate(100)(i => i)
  val m1: mutable.Map[Int, String] = mutable.Map(1 -> "a", 2 -> "b", 3 -> "c")
  val m2: mutable.Map[Int, String] =
    val m = mutable.Map.empty[Int, String]
    m(3) = "c"
    m(1) = "a"
    m(2) = "b"
    m
  val im: Map[Int, String] = (1 to 40).map(i => i -> i.toString).toMap
  val offset = 5
  val f: Int => Int = x => x + offset
  val holder = new Holder("h", Array(1, 2), f)

object M:
  def hashes(k: Expr[Int])(using Quotes): Expr[String] =
    val n = k.valueOrAbort
    val fresh = new Holder("fresh", Array(n), Values.f)
    val g: Int => Int = x => x + n
    val hs = List(Values.cyclic, Values.big, Values.m1, Values.m2, Values.im, Values.f, Values.holder, Color.Green, fresh, g, new Node(n, null))
      .map(System.identityHashCode)
    Expr(hs.mkString(" ") + " " + (System.identityHashCode(Values.m1) == System.identityHashCode(Values.m2)))

inline def hs(inline k: Int): String = ${ M.hashes('k) }
