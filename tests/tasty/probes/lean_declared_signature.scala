// jars: cats-core cats-kernel
package probes.leansig

// A selection of a jar member whose parameter names scala-library's `immutable.SortedSet`, which the
// lean std binds with `collection.SortedSet` to one class of its own: the selection keeps the
// declaration's signature (`TreePickler.pickleTree`'s SELECTin), `fromSet([1],
// scala.collection.immutable.SortedSet)`, which scalac's readTasty resolves.
object Use:
  def f(s: scala.collection.immutable.SortedSet[Int]) = cats.data.NonEmptySet.fromSet(s)
