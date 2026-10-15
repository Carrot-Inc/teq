// expect: 9:7: error: error overriding method fromProduct in trait Product
// expect: method fromProduct of type (p: Product): Int has incompatible type
// expect: 1 error found
// A companion's own `fromProduct` overrides its mirror parent's, checked as `RefChecks` does after
// `PostTyper`, against the `MirroredMonoType` alias `SyntheticMembers.addMirrorSupport` adds beside it
// (scalac: `C.MirroredMonoType`, E164).
case class C(x: Int)
object C:
  def fromProduct(p: Product): Int = 42
