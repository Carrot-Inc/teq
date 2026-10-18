object P {
  def test[N <: Null](f: ([A] => A => A) | N): [A] => A => A = f _
  def bottom[N <: Nothing](f: ([A] => A => A) | N): [A] => A => A = f _
  def main(args: Array[String]): Unit = {
    val f = [A] => (a: A) => a
    println(test[Null](f)(1))
    println(bottom[Nothing](f)("b"))
  }
}
