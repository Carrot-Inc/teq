// expect: 10:44: error: Found:    Int⏎Required: String
// expect: 13:53: error: value length is not a member of Any
// expect: 16:40: error: Not found: type nope
// expect: 20:53: error: Found:    "a"⏎Required: Int
// expect: 4 errors found
// A `for`'s alias that is a typed variable is a value definition: its right side is checked against the written
// type, its variable has that type, and a lowercase name of the type is the type of that name.
object Test:
  def mismatch: Int =
    val r = for (y <- List(1); l: String = y) yield l.length
    r.sum
  def declared: Int =
    val r = for (y <- List(1); l: Any = "st") yield l.length
    r.sum
  def unknownName: Int =
    val r = for (y <- List(1); l: List[nope] = List("a")) yield l.size
    r.sum
  def otherName: Int =
    type elem = Int
    val r = for (y <- List(1); l: List[elem] = List("a")) yield l.size
    r.sum
