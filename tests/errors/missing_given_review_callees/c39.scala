trait TC39[T]
object TC39 { def derived[T](using need: Numeric[String]): TC39[T] = new TC39[T] {} }

case class C39(i: Int) derives TC39
// scalac 3.8.4's line whole, at its place 4:32 (teq's 4:15: the row "the place")
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c39.scala:4:15: error: No given instance of type Numeric[String] was found for parameter need of method derived in object TC39
