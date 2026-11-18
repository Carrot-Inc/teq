// expect: values of types Int and Null cannot be compared with == or !=
// expect: 12:17: error: type mismatch: found 1, required AnyRef
// expect: 14:15: error: type mismatch: found Null, required T
// expect: 16:14: error: type mismatch: found Null, required Int
// expect: values of types Null and Boolean cannot be compared with == or !=
// teq: --strict-equality

class C

def a(x: Int): Boolean = x == null

val b: AnyRef = 1

def c[T]: T = null

def d: Int = null

def e(s: String, k: C, o: Option[Int]): Boolean = s == null && null == k && o != null

def f(x: Boolean): Boolean = null != x
