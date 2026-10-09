// Two lazy vals of an object count in an array the object made: each initialiser changes the
// array, which existed before the initialiser began, and gives it as its value. In scalac's order
// the second read counts on from the first.
object S:
  val shared: Array[Int] = Array(0)
  lazy val a: Array[Int] = { shared(0) += 1; shared }
  lazy val b: Array[Int] = { shared(0) += 1; shared }
