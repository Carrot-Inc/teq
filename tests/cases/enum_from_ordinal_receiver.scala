// A class-only enum's `fromOrdinal` is a method of its companion that always throws
// (`DesugarEnums.enumLookupMethods`): its receiver is evaluated once, before the argument and
// the lookup, whatever the cases, a companion that initialises printing first.
enum Shape:
  case Circle(r: Int)
  case Square(s: Int)
object Shape:
  println("static companion")

class O:
  println("outer")
  enum E:
    case B(x: Int)
  object E:
    println("companion")
  enum Mixed:
    case One
    case Two(x: Int)

def local(n: Int): String =
  enum L:
    case P(x: Int)
  object L:
    println("local companion " + n)
  try L.fromOrdinal(n).toString
  catch case e: NoSuchElementException => e.getMessage

def arg(i: Int): Int =
  println("argument " + i)
  i

@main def run(): Unit =
  try println(Shape.fromOrdinal(arg(0)))
  catch case e: NoSuchElementException => println(e.getMessage)
  try println((new O).E.fromOrdinal(arg(1)))
  catch case e: NoSuchElementException => println(e.getMessage)
  val o = new O
  try println({ println("receiver"); o.E }.fromOrdinal(arg(2)))
  catch case e: NoSuchElementException => println(e.getMessage)
  println({ println("mixed receiver"); o.Mixed }.fromOrdinal(arg(0)))
  try println(o.Mixed.fromOrdinal(arg(1)))
  catch case e: NoSuchElementException => println(e.getMessage)
  println(local(0))
  println(local(3))
