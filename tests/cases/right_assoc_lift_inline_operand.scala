// `typedInfixOp` decides on the argument as typed. A plain inline call is an application still to be inlined:
// impure, bound, and the binding stays when its expansion is a pure selection (the selection throws before the
// receiver is evaluated). A transparent inline call is expanded by its typing and is its expansion: a pure
// selection, not bound, evaluated after the receiver. Master bound both.
class Box(val value: Int)
inline def plain(box: Box): Int = box.value
transparent inline def shown(box: Box): Int = box.value
def right: List[Int] = { println("right"); Nil }
@main def run(): Unit =
  val box: Box = null
  try println(plain(box) :: right)
  catch case _: Throwable => println("caught")
  try println(shown(box) :: right)
  catch case _: Throwable => println("caught")
  val full = new Box(7)
  println(plain(full) :: right)
  println(shown(full) :: right)
