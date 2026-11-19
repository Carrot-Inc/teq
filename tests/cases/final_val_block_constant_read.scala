// A final val of no written type whose right-hand side is a block ending in a literal has the literal's constant
// type, and a read of it in the same object is the constant, the block still run where the val is defined.
object P {
  final val f3 = { println(3); 3 }
  val f4 = f3 + 1
  val t: 3 = f3
}
@main def main(): Unit = println(P.f4 + P.t)
