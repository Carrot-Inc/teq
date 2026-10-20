package a
// Plain inline calls, which the inlining phase expands once the program is typed without an error.
object Lib:
  inline def twice(x: Int): Int = x + x
  inline def greet: String = "hi " + twice(21)
@main def run(): Unit = println(Lib.greet)
