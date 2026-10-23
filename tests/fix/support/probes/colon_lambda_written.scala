object O:
  def take(f: Int => Int): Int = f(0)
  val result = take:
    (x: Int) => x
    1
@main def run(): Unit = println(O.result)
