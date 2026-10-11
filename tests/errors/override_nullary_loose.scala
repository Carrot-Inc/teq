// `def x` and `def x()` override each other only where one of them, or a member either overrides,
// is Java's, Any's, Object.clone or a Scala 2 class's (dotty's matchNullaryLoosely): a Scala class's
// `toString[T]()` is none of them, nor a trait's `h()`; scala-library's Iterator is a Scala 2 class.
// expect: 16:16: error: error overriding method toString in class P of type [T](): String: method toString of type [T]: String has incompatible type
// expect: 28:7: error: error overriding method h in trait W of type (): Int: method h of type Int has incompatible type
// expect: 2 errors found
class It extends Iterator[Int]:
  def hasNext(): Boolean = false
  def next(): Int = 0
class JIt extends java.util.Iterator[Int]:
  def hasNext: Boolean = false
  def next: Int = 0
class P:
  def toString[T](): String = "p"
class Q extends P:
  override def toString[T]: String = "q"
class R:
  override def toString(): String = "r"
class S extends R:
  override def toString: String = "s"
class U:
  override def clone(): AnyRef = this
class V extends U:
  override def clone: AnyRef = this
trait W:
  def h(): Int
class X extends W:
  def h: Int = 1
