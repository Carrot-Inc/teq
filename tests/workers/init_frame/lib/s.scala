// Two lazy locals of an object's initialiser count in a local and an array of their enclosing
// frame, which the object's initialiser made and its end stamped, since the closures the object
// keeps reach them: each lazy local's initialiser changes them, and they existed before it began.
// In scalac's order the second read counts on from the first. Upstream, in the shapes the TASTy
// writer states (an element's compound assignment and `new Array` are withheld from the products).
object S:
  val counters: (() => Int, () => Int) =
    var n = 0
    val counts = Array(0)
    lazy val a: Int = { n = n + 1; counts(0) = counts(0) + 1; n * 10 + counts(0) }
    lazy val b: Int = { n = n + 1; counts(0) = counts(0) + 1; n * 10 + counts(0) }
    (() => a, () => b)
