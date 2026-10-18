// A type alias spelt as a class's encoded name: the alias is the package object's member under its spelling, which
// is the class's key in the root package's scope (Namer.checkNoConflict), so it is refused, as scalac 3.8.4 refuses
// it ("a$minusb is already defined as class a-b"); teq words it as it words a plain `type A` beside `class A`.
class `a-b` { val value = 42 }
type a$minusb = String
// expect: 5:6: error: type a$minusb is already defined
