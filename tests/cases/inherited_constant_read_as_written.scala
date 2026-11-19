// A final val of no written type that overrides takes its right-hand side's constant type (a block ending in a
// literal too), and a read of it in the same object is that constant, as a written one is (its twin:
// inherited_constant_written_twin).
trait T {
  val f3: Int = { println("T.f3"); -3 }
  val f4: Int = { println("T.f4"); -4 }
}
object O extends T {
  override final val f3 = { println(3); 3 }
  override val f4 = f3 + 1
}
@main def main(): Unit =
  println(O.f4)
  val k: 3 = O.f3
  println(k)
