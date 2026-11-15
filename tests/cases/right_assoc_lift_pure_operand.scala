// An operand `typedInfixOp` does not lift is not bound: a pure expression (a selection on a value, the
// construction of a class without an initializer) is evaluated where the application reads it, after the receiver.
// With a null qualifier the receiver's effect comes first and the selection throws after it; master bound the
// selection ahead and threw first.
class Box(val value: Int)
class C
class P(val name: Int)
def right: List[Int] = { println("right"); Nil }
def rightC: List[C] = { println("right"); Nil }
def f(x: Int): Int = { println("left"); x }
@main def run(): Unit =
  val box: Box = null
  try println(box.value :: right)
  catch case _: Throwable => println("caught")
  println((new C :: rightC).size)
  val p = new P(3)
  println(p.name :: right)
  val acc = List(1)
  println(f(1) :: Nil)
  println(f(2) :: acc)
  println(p.name :: acc)
