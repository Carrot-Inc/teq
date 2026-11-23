// Local defs are units of their own: a local def that captures a value loops by a `return` (its own parameters
// go round, the capture stays); a def that loops through another receiver holds a local def that loops on its
// own; a def that loops on itself under a `return` holds a local def that walks other instances.
final class M(val id: Int, var next: M) {
  final def otherWithThisLocal(n: Int): Int = {
    def count(k: Int, acc: Int): Int = { if k > 0 then return count(k - 1, acc + 1); acc }
    if n > 0 then next.otherWithThisLocal(n - 1) else count(100000, id)
  }
  final def thisWithWalkLocal(n: Int): Int = {
    def walk(m: M, k: Int): Int = if k > 0 then walk(m.next, k - 1) else m.id
    if n > 0 then return thisWithWalkLocal(n - 1)
    walk(this, 4)
  }
}
object NL {
  final def nested(n: Int): Int = {
    def g(k: Int): Int = { if k > 0 then return g(k - 1); n * 10 }
    if n > 3 then return nested(n - 1)
    g(100000)
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    val a = new M(1, null); val b = new M(2, null); val c = new M(3, a)
    a.next = b; b.next = c
    println(a.otherWithThisLocal(2))
    println(a.thisWithWalkLocal(100000))
    println(NL.nested(100000))
  }
}
