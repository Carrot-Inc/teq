// A guard's brace block keeps its statements' separators inside the case region (a nested `InBraces`).
def f(x: Int): Int = x match
  case 1 if {
    val a = x + 1
    val b = a + 1
    b == 3
  } =>
    val c = x + 3
    c
  case _ => 0
@main def run(): Unit = println(f(1))
