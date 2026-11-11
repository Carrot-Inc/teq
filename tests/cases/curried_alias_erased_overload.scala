type Const[A] = [X] =>> A
object O:
  def f(x: Const[Int][String]): String = "int"
  def f(x: AnyRef): String = "ref"
@main def run(): Unit = println(O.f(1) + " " + O.f("s"))
