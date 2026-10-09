// Two lazy vals of an object count in its variable: each initialiser changes a field of the object,
// which existed before the initialiser began and which an earlier run may have initialised on one
// worker's heap and not on another's. In scalac's order the second read counts on from the first.
object S:
  var n = 0
  lazy val a: Int = { n += 1; n }
  lazy val b: Int = { n += 1; n }
