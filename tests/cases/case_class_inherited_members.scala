// A case class takes none of the rule's `toString`, `hashCode` and `equals` where a trait it
// extends defines them (dotty's `existingDef`), and a case object's `toString` is its own name
// whatever its `productPrefix` (`SyntheticMembers.toStringBody`).
trait T:
  override def toString: String = "inherited"
  override def hashCode: Int = 31
  override def equals(x: Any): Boolean = false
case class C(x: Int) extends T
case object O extends Product:
  override def productPrefix: String = "custom"

@main def run(): Unit =
  println(C(1))
  println(C(1).hashCode)
  println(C(1) == C(1))
  println(O)
  println(O.productPrefix)
