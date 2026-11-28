//> using platform jvm
// A case object that is its own `Mirror.Singleton` and overrides `fromProduct`: the user's method and the bridges
// erasure makes for it are the class's (dotty's `SyntheticMembers.addMirrorSupport` adds no second).
case object Lone extends scala.deriving.Mirror.Singleton {
  type MirroredMonoType = Lone.type
  type MirroredType = Lone.type
  type MirroredLabel = "Lone"
  override def fromProduct(p: Product): Lone.type = Lone
}
@main def run(): Unit =
  println(Lone.fromProduct(Tuple1(1)))
  println((Lone: scala.deriving.Mirror.Singleton).fromProduct(EmptyTuple))
