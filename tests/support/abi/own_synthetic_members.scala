// A user's own definition of a member the compiler also makes: the user's is the class's, with the bridges made
// for it, and no second (the copy family and a singleton's `fromProduct`; dotty's
// `Namer.invalidateIfClashingSynthetic` and `SyntheticMembers.addMirrorSupport`).
// abi: copy copy$default$1 copy$extension copy$default$1$extension fromProduct
package own_synthetic_members
case class Bump(x: Int) {
  def copy(x: Int = 7): Bump = new Bump(x + 1)
}
case class Boxed(x: Int) extends AnyVal {
  def copy(x: Int = 7): Boxed = new Boxed(x + 1)
}
case object Lone extends scala.deriving.Mirror.Singleton {
  type MirroredMonoType = Lone.type
  type MirroredType = Lone.type
  type MirroredLabel = "Lone"
  override def fromProduct(p: Product): Lone.type = Lone
}
