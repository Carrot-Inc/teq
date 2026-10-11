// The `hashCode` a case class takes by rule (dotty's `SyntheticMembers.chooseHashcode`): its
// name's hash without fields, the mix of its fields where one is of a primitive class, else
// `MurmurHash3.productHash`, which reads the class's own `productArity` and `productElement`.
case class R(s: String, t: String):
  override def productElement(n: Int): Any = "e" + n
case class P(i: Int, s: String):
  override def productElement(n: Int): Any = "e" + n
case class Z(s: String):
  override def productArity: Int = 0
case class Plain(s: String, t: String)
case object O

@main def run(): Unit =
  println(R("a", "b").hashCode == R("x", "y").hashCode)
  println(P(1, "a").hashCode == P(2, "b").hashCode)
  println(Z("x").hashCode == Z("y").hashCode)
  println(Plain("a", "b").hashCode == Plain("x", "y").hashCode)
  println(Plain("a", "b").hashCode)
  println(O.hashCode == "O".hashCode)
