// The mark is the annotation as typed, in the definition's own environment: an instance that does not type (an
// argument, a type argument the class does not take) marks nothing, and the member needs `override` as an unmarked
// one does; a type alias's annotation is read where the alias stands (an import inside the class body), and by its
// class through an alias of the annotation (E). scalac tells
// the annotation's own errors there (too many arguments; E053, no type parameters) and stops before its override
// checks; teq tells no annotation's own error yet, so the lines below are the members' as unmarked ones.
trait A { def f: Int = 1; def g: Int = 1 }
class B extends A { @scala.annotation.unchecked.uncheckedOverride(1) def f: Int = 2 }
class C extends A { @scala.annotation.unchecked.uncheckedOverride[Int] def g: Int = 2 }
trait Types { type T = Int }
class D extends Types {
  import scala.annotation.unchecked.uncheckedOverride
  @uncheckedOverride type T = String
}
type ann = scala.annotation.unchecked.uncheckedOverride
class E extends Types { @ann type T = String }
// expect: 8:74: error: method f needs `override` modifier to override method f in trait A
// expect: 9:76: error: method g needs `override` modifier to override method g in trait A
// expect: 13:27: error: error overriding type T in trait Types, which equals Int;
// expect: 16:35: error: error overriding type T in trait Types, which equals Int;
// expect: 4 errors found
