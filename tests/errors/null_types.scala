// expect: Values of types Int and Null cannot be compared with == or !=
// expect: 12:17: error: Found:    1⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 14:15: error: Found:    Null⏎Required: T
// expect: 16:14: error: Found:    Null⏎Required: Int
// expect: Values of types Null and Boolean cannot be compared with == or !=
// teq: --strict-equality

class C

def a(x: Int): Boolean = x == null

val b: AnyRef = 1

def c[T]: T = null

def d: Int = null

def e(s: String, k: C, o: Option[Int]): Boolean = s == null && null == k && o != null

def f(x: Boolean): Boolean = null != x
