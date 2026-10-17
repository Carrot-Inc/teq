trait C[T]
object P {
  def f[T: C](): Int = 1
  given C[Int] = new C[Int] {}
  val x = f[Int]()()
}
