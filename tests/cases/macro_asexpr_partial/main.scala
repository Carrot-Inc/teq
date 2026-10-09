import Macros.*

@main def run(): Unit =
  println(tried { val a = 2; a + 1 })
  println(tried(List("a", "b").mkString("-")))
  println(tried { val ok = true; ok && !ok })
