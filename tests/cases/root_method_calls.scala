// `toString`, `hashCode` and `equals` called in an inline body on a parameter whose declared type has none of its own:
// the expansion calls `Any`'s member, which reaches the runtime class's, or `java.lang.Object`'s where it defines none
// (a class name and a hash, the identity).
trait B
class K extends B
class N(val n: Int) extends B:
  override def toString: String = s"N($n)"
  override def hashCode: Int = n
  override def equals(that: Any): Boolean = that match
    case m: N => m.n == n
    case _ => false
inline def name(x: B): String = x.toString.takeWhile(_ != '@')
inline def same(x: B, y: B): Boolean = x.equals(y)
inline def hashed(x: B): Int = x.hashCode()
@main def run(): Unit =
  val k = new K
  println(name(k))
  println(name(new N(3)))
  println(hashed(k) == hashed(k))
  println(hashed(new N(7)))
  println(same(k, k))
  println(same(k, new K))
  println(same(new N(1), new N(1)))
