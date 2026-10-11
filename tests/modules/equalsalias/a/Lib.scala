package equalsalias

// A field of an alias of a primitive class is compared with the references
// (`SyntheticMembers.equalsBody`'s `typeSymbol`), through the products as from source: the
// products pickle the field's type as it is written.
type Count = Long

class Touch:
  override def equals(x: Any): Boolean = { println("visited"); true }

case class Aliased(a: Touch, n: Count)
case class Primitive(a: Touch, n: Long)
