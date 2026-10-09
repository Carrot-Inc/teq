// A spelled `*:` chain ending in the alias `EmptyTuple` erases to `Product` (`erasePair`): an
// inline method's parameter of that chain binds the argument at its own type, the chain, and its
// selection casts it to the tuple class it reads (`trySmallGenericTuple`), checked in the expansion.
inline def first(x: Int *: Int *: EmptyTuple): Int = x._1
inline def second(inline x: Int *: Int *: EmptyTuple): Int = x._2
@main def run(): Unit =
  try println(first(((1, 2, 3): Any).asInstanceOf[Int *: Int *: EmptyTuple]))
  catch case _: ClassCastException => println("CCE")
  try println(second(((1, 2, 3): Any).asInstanceOf[Int *: Int *: EmptyTuple]))
  catch case _: ClassCastException => println("CCE")
  println(first((4, 5)) + second((6, 7)))
