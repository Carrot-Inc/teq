// A type lambda's binder annotated with an argument that reads a local of the method: the annotation is kept
// by the binder's type parameter, which a worker makes. scalac 3.8.4 prints `1`.
class ann(val n: Any) extends scala.annotation.StaticAnnotation
def local(x: Int): Int = {
  type L = [@ann(x) A] =>> List[A]
  val l: L[Int] = List(x)
  l.head
}
@main def main(): Unit = println(local(1))
