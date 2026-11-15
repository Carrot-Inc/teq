// A cast the source writes is impure for `typedInfixOp`'s lift (`TreeInfo.exprPurity` calls the compiler's own
// cast pure, not `asInstanceOf`), and so is what stands on it: a selection, a constructor's argument, an element of
// an auto-tupled operand. The operand is bound ahead of the receiver and a failing cast throws before the receiver's
// effect. Master evaluated the receiver first for the auto-tupled operand.
class Box(val value: Int)
class Held(val v: String)
def right: List[Any] = { println("right"); Nil }
@main def run(): Unit =
  val x: Any = 1
  try println(x.asInstanceOf[String] :: right)
  catch case _: Throwable => println("caught")
  val y: Any = "s"
  try println(y.asInstanceOf[Box].value :: right)
  catch case _: Throwable => println("caught")
  try println(new Held(x.asInstanceOf[String]) :: right)
  catch case _: Throwable => println("caught")
  try println((x.asInstanceOf[String], 1) :: right)
  catch case _: Throwable => println("caught")
  println(y.asInstanceOf[String] :: right)
