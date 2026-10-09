// jars: scala-library
// std: scala-library
// An enum value whose constructor reads an earlier value through the companion (`case B extends E(E.A)`): the
// companion makes the values in order and stores each field as it is made, as scalac's (the JVM ABI's
// alignment with scalac's enums).
enum E(val previous: E):
  case A extends E(null)
  case B extends E(E.A)
  case C extends E(E.B)
object E:
  val chain = List(A, B, C).map(_.previous)
object Main:
  def main(args: Array[String]): Unit =
    println(E.B.previous == E.A)
    println(E.C.previous == E.B)
    println(E.A.previous)
    println(E.chain)
