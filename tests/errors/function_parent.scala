// expect: 5:7: error: class i0 needs to be abstract, since def apply(): Int in trait Function0 is not defined
// expect: 6:7: error: class E needs to be abstract, since def apply(v1: Int): 1 in trait Function1 is not defined
// expect: 7:9: error: object creation impossible, since def apply(v1: Int): Int in trait Function1 is not defined
// expect: 3 errors found
class i0 extends Function0[Int]
class E extends (Int => 1)
val f = new (Int => Int) {}
class Ok extends (Int => Int) { def apply(x: Int): Int = x }
