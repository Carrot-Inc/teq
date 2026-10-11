// A value case class's `equals` by rule asks a `canEqual` of its own last, and its `toString`
// reads its own `productPrefix` (dotty's `SyntheticMembers`).
case class V(x: Int) extends AnyVal:
  override def canEqual(that: Any): Boolean = false
case class W(x: Int) extends AnyVal:
  override def productPrefix: String = "Ww"
  override def productArity: Int = 1

@main def run(): Unit =
  println(V(1).canEqual(V(1)))
  println(V(1).equals(V(1)))
  println(W(2))
  println((V(3): Product).canEqual(V(3)))
  println(W(4) == W(4))
