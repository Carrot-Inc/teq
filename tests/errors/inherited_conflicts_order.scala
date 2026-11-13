// expect: 14:7: error: class Three inherits conflicting members: m2 in trait Early and m2 in trait Late; declare an override in class Three
// expect: 14:7: error: class Three inherits conflicting members: m3 in trait Early and m3 in trait Late; declare an override in class Three
// expect: 14:7: error: class Three inherits conflicting members: m1 in trait Early and m1 in trait Late; declare an override in class Three
// expect: 19:7: error: class Owners inherits conflicting members: y in trait Mid and y in trait Last; declare an override in class Owners
// expect: 19:7: error: class Owners inherits conflicting members: x in trait First and x in trait Mid; declare an override in class Owners
// expect: 23:7: error: class Names inherits conflicting members: g in trait Plain and g in trait Loaded; declare an override in class Names
// expect: 23:7: error: class Names inherits conflicting members: f in trait Plain and f in trait Loaded; declare an override in class Names
// expect: 7 errors found
// A class's inherited conflicts stand in the order scalac meets the pairs: the members of the trait mixed in
// last in that trait's order, then those of the trait before it, an overloaded name where its conflicting
// alternative is declared. scalac gives each class one error, for the first of them, and names the others.
trait Early { def m3: Int = 1; def m1: Int = 2; def m2: Int = 3 }
trait Late { def m2: Int = 4; def m3: Int = 5; def m1: Int = 6 }
class Three extends Early with Late

trait First { def x: Int = 1 }
trait Mid { def x: Int = 2; def y: Int = 2 }
trait Last { def y: Int = 3 }
class Owners extends First with Mid with Last

trait Plain { def g: Int = 1; def f(n: Int): Int = n; def f(s: String): Int = s.length }
trait Loaded { def f(d: Double): Int = 0; def g: Int = 2; def f(s: String): Int = 1 }
class Names extends Plain with Loaded
