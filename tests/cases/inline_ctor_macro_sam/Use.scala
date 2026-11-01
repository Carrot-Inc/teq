class A(val s: Int):
  val m = Mac.sam(s)
class B(override val s: Int) extends A(s + 1)
@main def run(): Unit = println(s"${B(2).m()} ${A(5).m()}")
