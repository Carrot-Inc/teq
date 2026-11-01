package base
class A(val s: Int):
  val saved = s
class B(override val s: Int) extends A(s + 1)
@main def run(): Unit = println(B(2).saved)
