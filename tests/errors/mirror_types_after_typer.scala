// expect: 16:16: error: type MirroredElemTypes not found
// expect: 16:36: error: type MirroredElemTypes not found
// expect: 17:15: error: type MirroredMonoType not found
// expect: 17:34: error: type MirroredMonoType not found
// expect: 21:12: error: type MirroredLabel not found
// expect: 24:12: error: type MirroredElemTypes not found
// expect: 25:12: error: type MirroredType not found
// expect: 26:12: error: type MirroredMonoType not found
// expect: 27:12: error: type MirroredMonoType not found
// expect: 9 errors found
// The mirror's members `SyntheticMembers.addMirrorSupport` gives a companion come after typing
// (`PostTyper`): no type member of `Mirror` is one of the companion's there, the `MirroredMonoType`
// it adds neither, inside the companion or through it.
case class P(x: Int)
object P:
  def elems(t: MirroredElemTypes): MirroredElemTypes = t
  def mono(p: MirroredMonoType): MirroredMonoType = p
sealed trait S
case class A(n: Int) extends S
object S:
  type L = MirroredLabel
case object O
object Use:
  type E = P.MirroredElemTypes
  type T = S.MirroredType
  def f(x: P.MirroredMonoType): Int = x.x
  type U = O.MirroredMonoType
