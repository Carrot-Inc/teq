// expect: 10:26: warning: method toStream in trait IterableOnceOps is deprecated since 2.13.0: Use .to(LazyList) instead of .toStream
// expect: 11:25: warning: method toStream in trait IterableOnceOps is deprecated since 2.13.0: Use .to(LazyList) instead of .toStream
// expect: 2 warnings found, errors under --werror
// teq: --werror --deprecation
// A deprecated member called through `super` and through `this` in a trait. scalac warns of the call through
// `this` alone: where it checks, the call through `super` stands behind the trait's super accessor. The first
// warning expected here is one scalac does not give; a reference is to be checked where scalac checks it, and
// that line is to go then.
trait Leading extends Iterator[Int] {
  def leadBySuper: Int = super.toStream.head
  def leadByThis: Int = this.toStream.head
}
class Tally(limit: Int) extends Leading {
  private var at = 0
  def hasNext: Boolean = at < limit
  def next(): Int = { at += 1; at + 2 }
}
object Main {
  def main(a: Array[String]): Unit = { println(new Tally(3).leadBySuper); println(new Tally(3).leadByThis) }
}
