package ndb2

// A by-name default that a later clause's default reads: the later getter takes the earlier argument
// again, its tree copied (`Applications.spliceMeth`), and every copy is the getter of the companion of
// the creation's prefix (`Applications.findDefaultGetter`), through `new`, the constructor's proxy and a
// written companion's `apply` alike, evaluated at each read.
class Outer(val n: Int) {
  var reads = 0
  class Item(a: => Int = { reads += 1; n })(val b: Int = a + a)
  case class Pair(x: Int)(y: => Int = { reads += 1; n + x })(val z: Int = y * 2)
  object Pair {
    def tag = "pair"
  }
}
