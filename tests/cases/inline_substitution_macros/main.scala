// The splices of macros run from the stored lambda, each expansion's copy of it: the reflected
// types, tree shapes, symbols and `Position.ofMacroExpansion` of direct calls and of calls an
// inline method of another file wraps, its parameters substituted. scalac prints the lines of
// the .expected file.
import M.*, W.*
@main def run(): Unit =
  val k = 4
  println(typeOf(List(1, 2)))
  println(typeOf[Any](3))
  println(shape(k + 1))
  println(shape(k * 2 - 1))
  println(twiceShape(k))
  println(pairType(1))
  println(pairType(true))
  println(where(k))
  println(whereWrapped(k + 1))
  println(symbolOf(k))
  println(symWrapped(k))
