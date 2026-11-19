object O52 { transparent inline def f: Numeric[String] ?=> Int => Int = (n: Numeric[String]) ?=> (i: Int) => i }

val result52 = O52.f.apply(1)
// scalac 3.8.4's line whole, at its place 3:27 (teq's 3:16: the row "the place")
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c52.scala:3:16: error: No given instance of type Numeric[String] was found for parameter of (Numeric[String]) ?=> Int => Int
