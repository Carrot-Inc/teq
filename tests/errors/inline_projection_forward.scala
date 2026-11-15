// expect: 15:31: error: forward reference to Mid (defined on line 17) extends over the definition of got (on line 16)
// expect: 21:31: error: forward reference to Mid (defined on line 23) extends over the definition of got (on line 22)
// expect: 27:35: error: forward reference to Mid (defined on line 29) extends over the definition of got (on line 28)
// expect: 33:35: error: forward reference to Mid (defined on line 35) extends over the definition of got (on line 34)
// expect: 39:31: error: forward reference to Mid (defined on line 41) extends over the definition of got (on line 40)
// expect: 45:32: error: forward reference to Mid (defined on line 47) extends over the definition of got (on line 46)
// expect: 52:37: error: forward reference to Mid (defined on line 54) extends over the definition of got (on line 53)
// expect: 58:31: error: forward reference to Mid (defined on line 60) extends over the definition of got (on line 59)
// expect: 8 errors found
// A creation that scalac's inliner keeps is in the tree where forward references are checked, inside an
// expansion as anywhere: a selection on it inside an argument, in a lambda, in the argument of a plain call,
// under an ascription, and a by-name parameter's argument name the class of the block that is defined later.
object Test:
  def insideArgument(n: Int): Int =
    def first: Int = pick(new Mid(1).v + 1)
    val got: Int = first
    class Mid(val v: Int)
    inline def pick(x: Int): Int = x + 1
    got
  def insideInlineArgument(n: Int): Int =
    def first: Int = pick(new Mid(1).v + 1)
    val got: Int = first
    class Mid(val v: Int)
    inline def pick(inline x: Int): Int = x + 1
    got
  def inLambda(n: Int): Int =
    def first: Int = app(x => new Mid(x).v)
    val got: Int = first
    class Mid(val v: Int)
    inline def app(f: Int => Int): Int = f(1)
    got
  def inInlineLambda(n: Int): Int =
    def first: Int = app(x => new Mid(x).v)
    val got: Int = first
    class Mid(val v: Int)
    inline def app(inline f: Int => Int): Int = f(1)
    got
  def byName(n: Int): Int =
    def first: Int = pick(new Mid(1).v)
    val got: Int = first
    class Mid(val v: Int)
    inline def pick(x: => Int): Int = x + 1
    got
  def ascribed(n: Int): Int =
    def first: Int = pick((new Mid(1).v): Int)
    val got: Int = first
    class Mid(val v: Int)
    inline def pick(x: Int): Int = x + 1
    got
  def inPlainCall(n: Int): Int =
    def twice(k: Int): Int = k * 2
    def first: Int = pick(twice(new Mid(1).v))
    val got: Int = first
    class Mid(val v: Int)
    inline def pick(x: Int): Int = x + 1
    got
  def secondList(n: Int): Int =
    def first: Int = pick(new Mid(1)(2).v)
    val got: Int = first
    class Mid(val v: Int)(val w: Int)
    inline def pick(x: Int): Int = x + 1
    got
  def main(args: Array[String]): Unit = println(insideArgument(1))
