import Macros.*

@main def run(): Unit =
  println(mapped {
    type Num = Int
    val n: Num = 2
    n + 1
  })
  println(mapped {
    type Label = String
    val parts: List[Label] = List("a", "b")
    parts.mkString("-")
  })
