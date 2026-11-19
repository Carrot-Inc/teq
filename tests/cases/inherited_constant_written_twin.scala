// A final val of a written constant type that overrides, read in the same object as its constant (the twin of
// inherited_constant_read_as_written, whose type is inferred).
trait T {
  val f3: Int = { println("T.f3"); -3 }
  val f4: Int = { println("T.f4"); -4 }
}
object O extends T {
  override final val f3: 3 = { println(3); 3 }
  override val f4 = f3 + 1
}
@main def main(): Unit =
  println(O.f4)
  val k: 3 = O.f3
  println(k)
