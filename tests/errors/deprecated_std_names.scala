// expect: 12:12: warning: type Stream in package scala is deprecated since 2.13.0: Use LazyList instead of Stream
// expect: 13:16: warning: value Stream in package scala is deprecated since 2.13.0: Use LazyList instead of Stream
// expect: warning: class Stream in package scala.collection.immutable is deprecated since 2.13.0: Use LazyListIterable (which is fully lazy) instead of Stream (which has a lazy tail only)
// expect: warning: object Stream in package scala.collection.immutable is deprecated since 2.13.0: Use LazyList (which is fully lazy) instead of Stream (which has a lazy tail only)
// expect: 16:12: warning: type Traversable in package scala is deprecated since 2.13.0: Use Iterable instead of Traversable
// expect: warning: method toStream in trait IterableOnceOps is deprecated since 2.13.0: Use .to(LazyList) instead of .toStream
// expect: 6 warnings found, errors under --werror
// teq: --werror --deprecation
// Each name of a deprecated definition of the std is warned of by its own mark, with scala-library's text: a
// type, a value, a class and an object by their paths, another alias, a member.
object Names {
  def a(s: Stream[Int]): Int = 1
  def b: Int = Stream(1).head
  def c(s: scala.collection.immutable.Stream[Int]): Int = 2
  def d: Int = scala.collection.immutable.Stream.empty[Int].length
  def e(t: Traversable[Int]): Int = 3
  def f: Int = List(1).toStream.head
}
