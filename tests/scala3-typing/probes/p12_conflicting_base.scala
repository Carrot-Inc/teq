trait A[T]:
  def f: T
trait B1 extends A[Int]:
  def f: Int = 1
trait B2 extends A[String]:
  def f: String = "s"
class D extends B1, B2
@main def run(): Unit = println(D().f)
