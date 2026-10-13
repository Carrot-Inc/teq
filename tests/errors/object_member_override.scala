// expect: 13:28: error: error overriding method x in trait A of type Int: object x has incompatible type
// expect: 13:48: error: object z needs `override` modifier to override method z in trait A
// expect: 17:14: error: error overriding method C in trait AA of type Int => AA.this.C: object C has incompatible type
// expect: 3 errors found
// absent: 13:38: error
// absent: 13:58: error
// A member object overrides as a value of its type (`RefChecks.checkOverride`): it implements an
// abstract member its type conforms to (`y`, `v`), is no `Int` (`x`), needs `override` over a
// concrete member (`z`); so does the companion a case class nested in a class is given, which
// is an object of its owner (scala/neg/t3137). An object over an inherited object is the
// namer's error (E069) alone.
trait A { def x: Int; def y: Any; def z: Any = 1; val v: Any }
class B extends A { object x; object y; object z; object v }

trait AA { type C; def C: Int => C }
class BB extends AA {
  case class C(v: Int)
}
