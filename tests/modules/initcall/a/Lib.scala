package icalla

// A call of a top-level def initialises its file's eager vals before its argument is evaluated,
// though the program never reads one of them.
val registry: List[String] =
  println("registry")
  List("a", "b")
val literal = 1
def twice(n: Int): Int = n * 2
