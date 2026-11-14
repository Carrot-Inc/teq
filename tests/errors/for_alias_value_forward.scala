// expect: 10:41: error: forward reference to l extends over the definition of l (on line 10)
// expect: 13:41: error: forward reference to l extends over the definition of l (on line 13)
// expect: 18:18: error: forward reference to l extends over the definition of l (on line 18)
// expect: 3 errors found
// A `for`'s alias that is a typed variable is in scope in its own initialiser, as a block's value is: reading it
// there is a forward reference that extends over its own definition.
object Test:
  def outer: Int =
    val l = 10
    val r = for (y <- List(1); l: Int = l + y) yield l
    r.sum
  def alone: Int =
    val r = for (y <- List(1); l: Int = l + y) yield l
    r.sum
  def leading: Int =
    val r =
      for
        l: Int = l + 1
        y <- List(1)
      yield l + y
    r.sum
