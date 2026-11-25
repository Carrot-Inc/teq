// A right-associative extension called as a method, its first-written argument a val selected through a null
// reference: the selection throws before the later argument is evaluated, as written.
class Box(val value: Int)
class E:
  extension (l: Int)
    def +:(r: Int): Int = { println("body"); l + r }
def n(): Int = { println("left"); 1 }
@main def run(): Unit =
  val e = new E
  val box: Box = null
  try println(e.+:(box.value)(n()))
  catch case _: NullPointerException => println("caught")
