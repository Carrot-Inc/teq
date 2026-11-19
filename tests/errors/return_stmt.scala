// expect: return outside method definition
// expect: method f has a return statement; it needs a result type
// expect: 8:33: error: Found:    "s"⏎Required: Int

object E:
  val x = { return 1 }
  def f(n: Int) = { if (n > 0) return 1; 2 }
  def g(n: Int): Int = { return "s" }
  class C { val y = if (true) return 2 else 3 }
