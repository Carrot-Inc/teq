def twice(): Int =
  val f = same { def early(x: Int): Int = { if x > 0 then return x + 1; 0 }; (y: Int) => early(y) }
  val a = f(41)
  println("after " + a)
  a + f(-1)
@main def run(): Unit = println(twice())
