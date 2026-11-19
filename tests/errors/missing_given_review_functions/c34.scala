def use34(f: Numeric[String] ?=> Int => Int): Int = f.apply(1)
// scalac 3.8.4's line whole, at its place 1:60 (teq's 1:53: the row "the place")
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c34.scala:1:53: error: No given instance of type Numeric[String] was found for parameter of (Numeric[String]) ?=> Int => Int
