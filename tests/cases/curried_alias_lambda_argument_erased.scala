type Second[A] = [B] =>> B
object O:
  def f(x: Second[String][Int]): String = "int"
  def f(x: AnyRef): String = "ref"
@main def run(): Unit = println(O.f(1) + " " + O.f("s"))
