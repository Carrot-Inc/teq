// `fromOrdinal` of an enum with class cases (dotty's `DesugarEnums.enumLookupMethods`): every enum
// has it, `values` and `valueOf` only one whose cases are all values; it returns the value case of
// the ordinal and throws for a class case's ordinal as for an ordinal of no case, for an enum of
// the top level, of a block and of a class.
enum Shape:
  case Dot
  case Circle(r: Double)
  case Line

def local(n: Int): String =
  enum E:
    case A
    case B(x: Int)
    case C
  val tries = (0 to 3).map: i =>
    try E.fromOrdinal(i).toString
    catch case e: NoSuchElementException => e.getMessage
  s"${E.fromOrdinal(0).ordinal} ${E.fromOrdinal(2).ordinal} ${tries.mkString(",")} $n"

class Holder:
  enum Kind:
    case K1
    case K2(v: Int)
    case K3

@main def run(): Unit =
  println(Shape.fromOrdinal(0))
  println(Shape.fromOrdinal(2))
  try println(Shape.fromOrdinal(1))
  catch case e: NoSuchElementException => println(e.getMessage)
  try println(Shape.fromOrdinal(7))
  catch case e: NoSuchElementException => println(e.getMessage)
  println(local(5))
  val h = new Holder
  println(h.Kind.fromOrdinal(2) eq h.Kind.K3)
  try println(h.Kind.fromOrdinal(1))
  catch case e: NoSuchElementException => println(e.getMessage)
