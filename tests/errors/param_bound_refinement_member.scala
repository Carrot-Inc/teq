// A type parameter does not conform to a refinement where the member in its bound's refinement is another one: a
// result outside the one asked, another name, a `def` where a `val` is asked, other parameters; nor where it has a
// lower bound alone, a class bound without the member, or no bound; nor through a parameter inside another one's
// bound (a side of an intersection, a refinement's parent, an alias's argument) whose own bound gives another
// result; nor where the bound's member takes a parameter of another erased type than the one asked (the call
// goes by reflection); nor a `val` asked of the bound's `var`, or a `var` of its `val`. scalac 3.8.4 refuses the
// same twenty lines.
// expect: 33:85: error: type mismatch: found T, required AnyRef{def self: Int}
// expect: 34:85: error: type mismatch: found T, required AnyRef{def self: String}
// expect: 35:81: error: type mismatch: found T, required AnyRef{def self: Int}
// expect: 36:80: error: type mismatch: found T, required AnyRef{val self: Int}
// expect: 37:103: error: type mismatch: found T, required AnyRef{def add(n: Int): Int}
// expect: 38:94: error: type mismatch: found T, required AnyRef{def self(n: Int): Int}
// expect: 39:93: error: type mismatch: found F[String], required AnyRef{def self: Int}
// expect: 40:61: error: type mismatch: found T, required AnyRef{def self: Int}
// expect: 41:60: error: type mismatch: found T, required AnyRef{def self: Int}
// expect: 42:50: error: type mismatch: found T, required AnyRef{def self: Int}
// expect: 43:89: error: type mismatch: found U, required AnyRef{def self: Int}
// expect: 44:97: error: type mismatch: found U, required AnyRef{def self: Int}
// expect: 45:108: error: type mismatch: found U, required AnyRef{def self: Int}
// expect: 46:116: error: type mismatch: found F[String], required AnyRef{def self: Int}
// expect: 47:98: error: type mismatch: found U, required AnyRef{def self: Int}
// expect: 48:96: error: type mismatch: found T, required AnyRef{def self(x: String): Int}
// expect: 49:79: error: type mismatch: found T, required AnyRef{val self: Int}
// expect: 50:79: error: type mismatch: found T, required AnyRef{val self: Int}
// expect: 51:92: error: type mismatch: found F[Int], required AnyRef{val self: Int}
// expect: 52:101: error: type mismatch: found U, required AnyRef{val self: Int}
// expect: 20 errors found
class Knot { def self: Int = 1 }
class Other
trait Named
type Id[A] = A
def otherResult[T <: AnyRef { def self: String }](t: T): AnyRef { def self: Int } = t
def widerResult[T <: AnyRef { def self: Any }](t: T): AnyRef { def self: String } = t
def otherName[T <: AnyRef { def other: Int }](t: T): AnyRef { def self: Int } = t
def defForVal[T <: AnyRef { def self: Int }](t: T): AnyRef { val self: Int } = t
def otherParameters[T <: AnyRef { def add(s: String): Int }](t: T): AnyRef { def add(n: Int): Int } = t
def fewerParameters[T <: AnyRef { def self: Int }](t: T): AnyRef { def self(n: Int): Int } = t
def otherArgument[F[X] <: AnyRef { def self: X }](t: F[String]): AnyRef { def self: Int } = t
def lowerAlone[T >: Knot](t: T): AnyRef { def self: Int } = t
def noMember[T <: Other](t: T): AnyRef { def self: Int } = t
def noBound[T](t: T): AnyRef { def self: Int } = t
def chained[T <: AnyRef { def self: String }, U <: T](u: U): AnyRef { def self: Int } = u
def inSides[T <: AnyRef { def self: String }, U <: T & Named](u: U): AnyRef { def self: Int } = u
def asParent[T <: AnyRef { def self: String }, U <: T { def more: Int }](u: U): AnyRef { def self: Int } = u
def appliedInSides[G[X] <: AnyRef { def self: X }, F[X] <: G[X] & Named](t: F[String]): AnyRef { def self: Int } = t
def throughAlias[T <: AnyRef { def self: String }, U <: Id[T]](u: U): AnyRef { def self: Int } = u
def erased[T <: AnyRef { def self(x: Any): Int }](t: T): AnyRef { def self(x: String): Int } = t
def valOfVar[T <: AnyRef { var self: Int }](t: T): AnyRef { val self: Int } = t
def varOfVal[T <: AnyRef { val self: Int }](t: T): AnyRef { var self: Int } = t
def appliedValOfVar[F[X] <: AnyRef { var self: X }](t: F[Int]): AnyRef { val self: Int } = t
def insideValOfVar[T <: AnyRef { var self: Int }, U <: T & Named](u: U): AnyRef { val self: Int } = u
