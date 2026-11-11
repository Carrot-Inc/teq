// A downstream of a/ as tests/modules builds it. a/'s inline `n7` summons `Ordering[T]`, `T` an object's alias of Int
// imported, and `n8` `Ordering[Int]`. scalac 3.8.4 prints the .expected, and so do teq's runs. a/'s products of the
// Scala.js check build over scala-library pickle `n7`'s summoned type as `Long[O.T]`, and b/'s check build fails:
// "value compare of Long[Int]" (tests/modules' check). scalac over the lean products also fails on
// `compiletime$package.summonInline` (scalac-lean-b), the owner products_lean_constvalue_owner pins. The same before
// the SELECTtpt port (tip34). Found by reviewer #14 of the workers exports helper (push 9).
package nb7

@main def run(): Unit =
  println(np.sub.Pk.n7(3))
  println(np.sub.Pk.n8(0))
