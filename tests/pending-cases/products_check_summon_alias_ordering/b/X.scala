// A downstream of a/ as tests/modules builds it. a/'s inline `n7`
// summons `Ordering[T]`, `T` an object's alias of Int imported,
// and `n8` `Ordering[Int]`. scalac 3.8.4 prints the .expected.
// teq's Scala.js check build of b/ over a/'s products is to pass
// (tests/modules' check). teq's lean products are to be ones scalac
// types b/ over (scalac-lean-b; products_lean_constvalue_owner).
package nb7

@main def run(): Unit =
  println(np.sub.Pk.n7(3))
  println(np.sub.Pk.n8(0))
