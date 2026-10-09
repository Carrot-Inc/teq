// The second code pass's `collection_partial_failure` program: `ArrayList.removeAll` is the JDK's
// `batchRemove`, whose removals before a throwing `contains` stand.
class Membership extends java.util.AbstractCollection[String]:
  def size(): Int = 1
  def iterator(): java.util.Iterator[String] =
    val one = new java.util.ArrayList[String]()
    one.add("a")
    one.iterator()
  override def contains(x: Any): Boolean =
    if x == "b" then throw new IllegalArgumentException("stop")
    x == "a"
@main def run(): Unit =
  val xs = new java.util.ArrayList[String]()
  xs.add("a"); xs.add("b"); xs.add("c")
  try xs.removeAll(new Membership) catch case _: IllegalArgumentException => println("stopped")
  println(xs)
