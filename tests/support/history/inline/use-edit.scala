object Use:
  val n: Int = Lib.f(1)
  def m: Int = n + 2
  def other: String = "unrelated"
@main def run(): Unit = println(Use.m)
