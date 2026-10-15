// A companion that declares `MirroredMonoType` abstractly gets the alias `SyntheticMembers`' `monoType` enters (the
// class), against which its own `fromProduct` is read.
case class C(x: Int)
object C:
  type MirroredMonoType
  def fromProduct(p: Product): C = new C(42)
@main def run(): Unit = println(C.fromProduct(Tuple1(1)))
