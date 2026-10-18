object P {
  def test[F <: [A] => A => A](f: F) = f _
  trait C { type F <: [A] => A => A; val f: F; def g = f _ }
  def nullable(h: ([A] => A => A) | Null) = h _
  def main(args: Array[String]): Unit = {
    val f = [A] => (a: A) => a
    println(test(f)(1))
    val c = new C { type F = [A] => A => A; val f = [A] => (a: A) => a }
    println(c.g("x"))
    println(nullable(f) != null)
  }
}
