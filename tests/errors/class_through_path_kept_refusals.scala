import scala.reflect.Selectable.reflectiveSelectable
class Ops:
  object nums:
    class Box(val v: Int)
object Factory:
  inline def empty: Int =
    class Outer:
      class Leaf
    val o = new Outer
    Array.empty[o.Leaf].length
def uses(o: Ops): Unit =
  val s: { def take(b: o.nums.Box): Int } = new { def take(b: o.nums.Box): Int = b.v + 1 }
  println(s.take(new o.nums.Box(1)))
  println(Factory.empty)
// Left as they are refused today, each by a named test: the tag of a class of a local class inside an inline
// method's body (the body's classes are copied where it expands, and the JVM's array class of the definition's
// is not there), and a structural call whose parameter is a class named through a path (the same call on a plain
// class does not find its method on the JVM).
// expect: 10:5: error: No ClassTag available for o.Leaf
// expect: 13:11: error: Structural access not allowed on method take because it has a parameter type with an unstable erasure
