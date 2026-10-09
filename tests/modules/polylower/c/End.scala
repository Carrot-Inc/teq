package plw
object End:
  def use(f: [A >: String] => (a: A) => A) = Mid.forward(f)
  def main(args: Array[String]): Unit =
    println(use([A >: String] => (a: A) => a)[Any]("lower"))
    // A literal's own bounds, which its body reads.
    val inc = [A <: Int] => (a: A) => a + 1
    println(inc[Int](41))
