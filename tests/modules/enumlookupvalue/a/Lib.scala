package enumlookupvalue

// An enum companion's `valueOf` and `fromOrdinal` referred to without arguments, the function of the
// argument, in bodies a downstream module reads from the products: a method's, a val's, an inline
// method's and a transparent inline method's, whose expansion the pickle holds with the lookup's call
// under the closure (`Tone.valueOf(x)`, the enum stated as for the applied call).
enum Tone:
  case Low, High

enum Shape:
  case Dot
  case Box(side: Int)

object Readers:
  def all(names: List[String]): List[Tone] = names.map(Tone.valueOf)
  val byOrdinal: Int => Tone = Tone.fromOrdinal
  inline def inlined(names: List[String]): List[Tone] = names.map(Tone.valueOf)
  transparent inline def shown(names: List[String]): List[Tone] = names.map(Tone.valueOf)
  transparent inline def shapes(ordinals: List[Int]): List[Shape] = ordinals.map(Shape.fromOrdinal)
  def viaInline(names: List[String]): List[Tone] = inlined(names)
  def viaTransparent(names: List[String]): List[Tone] = shown(names)
