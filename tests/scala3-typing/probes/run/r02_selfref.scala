object T:
  val a = 42
  def main(): Unit =
    val a: Int = a
    println(a)
@main def run(): Unit = T.main()
