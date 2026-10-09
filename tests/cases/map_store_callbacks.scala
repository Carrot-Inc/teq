// std: lean
// The store runs a key's `hashCode` once per lookup, insertion or removal, and none during a
// compaction, so a `hashCode` that removes another key from the map under compaction cannot
// restart it; the tombstone of a removed entry is the std's own, so no element can pass for
// one; a slice with an inverted or overflowing interval is empty.
object Counter:
  var hashes = 0
  var equalities = 0

final class K(val n: Int):
  override def hashCode: Int =
    Counter.hashes += 1
    n
  override def equals(x: Any): Boolean = x match
    case k: K =>
      Counter.equalities += 1
      n == k.n
    case _ => false
  override def toString: String = "K" + n

object State:
  var parent = Map.empty[Any, Int]
  var armed = false
  var calls = 0

final class Reentrant:
  override def hashCode: Int =
    if State.armed then
      State.calls += 1
      if State.calls > 10 then throw new RuntimeException("recursive compaction")
      State.parent.removed(39)
    0
  override def equals(x: Any): Boolean = x match
    case _: Reentrant => true
    case _ => false

@main def main(): Unit =
  var m = Map.empty[K, Int]
  var i = 0
  while i < 40 do
    m = m.updated(new K(i), i)
    i += 1
  m = m.updated(new K(0), -1)
  Counter.hashes = 0
  Counter.equalities = 0
  val got = m.get(new K(5))
  println("get " + got + " hashes " + Counter.hashes + " equalities " + Counter.equalities)
  Counter.hashes = 0
  Counter.equalities = 0
  val has = "" + m.contains(new K(7)) + " " + m.contains(new K(70)) + " " + m.getOrElse(new K(8), -1)
  println("contains " + has + " hashes " + Counter.hashes)
  Counter.hashes = 0
  val added = m.updated(new K(100), 100)
  val removed = added.removed(new K(3))
  println("update " + added.size + " " + removed.size + " hashes " + Counter.hashes)
  // A removal that compacts: the parent's keys are not hashed again.
  var big = Map.empty[K, Int]
  i = 0
  while i < 60 do
    big = big.updated(new K(i), i)
    i += 1
  i = 0
  while i < 30 do
    big = big.removed(new K(i))
    i += 1
  Counter.hashes = 0
  Counter.equalities = 0
  val compacted = big.removed(new K(30))
  println("compaction " + compacted.size + " " + compacted.contains(new K(59)) + " " + compacted.contains(new K(30)) + " hashes " + Counter.hashes)
  // The reentrant case: a hash that removes another key from the parent.
  var parent = Map.empty[Any, Int]
  parent = parent.updated(new Reentrant, 0)
  i = 1
  while i < 40 do
    parent = parent.updated(i, i)
    i += 1
  i = 1
  while i <= 20 do
    parent = parent.removed(i)
    i += 1
  State.parent = parent
  State.armed = true
  val child = parent.removed(38)
  State.armed = false
  println("reentrant " + child.size + " " + State.calls + " " + child.contains(39))
  // A set whose element is the tombstone's class cannot be made from user code; an element
  // equal to nothing else survives the removal of another.
  val s = Set[Any](new Reentrant, 1, 2) - 1
  println("set " + s.size + " " + s.contains(new Reentrant))
  println("slice " + Vector(1, 2, 3).slice(1, Int.MinValue).toString + " " + Vector(1, 2, 3).slice(2, 1) + " " + Vector(1, 2, 3).slice(Int.MinValue, 2))
