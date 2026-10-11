// `SyntheticMembers.equalsBody` orders the fields by the declared type's symbol: a field of an
// alias of `Long` or of an opaque type over `Int` is compared with the references, in declaration
// order, so `a`'s `equals` runs first; a field of `Long` itself is compared first. The hash sees
// through the alias (`chooseHashcode`'s `classSymbol`): the fields' mix of a primitive field.
type Count = Long

class Touch:
  override def equals(x: Any): Boolean = { println("visited"); true }
  override def hashCode: Int = 7

case class Aliased(a: Touch, n: Count)
case class Primitive(a: Touch, n: Long)

object Types:
  opaque type Age = Int
  def age(n: Int): Age = n
  case class Opaque(a: Touch, n: Age)
  def test(): Unit =
    println(Opaque(new Touch, age(1)) == Opaque(new Touch, age(2)))

@main def run(): Unit =
  println(Aliased(new Touch, 1L).equals(Aliased(new Touch, 2L)))
  println(Primitive(new Touch, 1L).equals(Primitive(new Touch, 2L)))
  println(Aliased(new Touch, 1L).hashCode)
  println(Primitive(new Touch, 1L).hashCode)
  Types.test()
