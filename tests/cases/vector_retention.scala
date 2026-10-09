// std: lean
// A version never writes into an array another version can reach, so a held vector or map
// keeps its own elements and no more: the backing array of a one-element vector stays one
// element long while ten thousand appends derive from it (`unsafeArray` is the std's own
// accessor of that array, so the expectation is written by hand), a traversal's predicate
// that appends does not extend what the traversal sees, two branches of one version do not
// see each other's element, a slice past the end is empty, and the same holds for the entries
// of a map and a set held across the updates made from them.
final class Probe(val n: Int)

@main def main(): Unit =
  val held = Vector(0)
  val backing = held.unsafeArray
  var current = held
  var i = 1
  while i <= 10000 do
    current = current :+ i
    i += 1
  println(held.length + " " + backing.length + " " + current.length + " " + held.unsafeArray.length)
  val one = Vector(1)
  def appending[B](x: B): B =
    val child = one :+ 2
    x
  println("" + one.exists(x => appending(x) == 2) + " " + one.map(x => appending(x)).length + " " + one.filter(x => appending(true)).length + " " + one)
  val base = Vector(1, 2, 3)
  val left = base :+ 4
  val right = base :+ 5
  println(left.toString + " " + right + " " + base + " " + (left.updated(0, 9) eq left) + " " + base.updated(1, 7) + " " + base)
  val big = Vector.tabulate(100)(i => i)
  val a = big :+ -1
  val b = big :+ -2
  println(a.last + " " + b.last + " " + big.length + " " + a.length + " " + (big.take(40) :+ 1).length + " " + big.take(40).last)
  println(Vector(1, 2, 3).slice(1, Int.MinValue).toString + " " + Vector(1, 2, 3).slice(2, 1) + " " + Vector(1, 2, 3).slice(-5, 2) + " " + Vector(1, 2, 3).slice(1, 99))
  var m = Map(0 -> new Probe(0))
  val heldMap = m
  i = 1
  while i <= 5000 do
    m = m.updated(i, new Probe(i))
    i += 1
  var s = Set(0)
  val heldSet = s
  i = 1
  while i <= 5000 do
    s = s + i
    i += 1
  println(heldMap.size + " " + m.size + " " + heldMap.toList.length + " " + heldSet.size + " " + s.size + " " + heldSet.toList.length + " " + heldMap.contains(4999) + " " + heldSet.contains(4999))
