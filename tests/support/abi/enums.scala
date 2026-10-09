// Item 6 of the JVM ABI alignment: an enum's companion holds the values
// as public static final enum fields (`E$.A`), through which the program reads them as a build over its products
// does, `$values` where every case is a value, and is the enum's `Mirror.Sum`: `ordinal(E)I` and its bridge.
// A value whose constructor reads an earlier one (`case B extends Linked(Linked.A)`): each made and stored in order.
// abi: E$#A E$#B Kind$#C Kind$#Small Mixed$#A $values values valueOf fromOrdinal E$#ordinal Mixed$#ordinal Kind$#ordinal scala/deriving/Mirror$Sum Linked$#A Linked$#B
package enums
enum E { case A, B }
enum Mixed { case A; case B(x: Int) }
enum Kind:
  case Small
  case Sized(n: Int)
  case C
object Kind:
  val all = List(Small, C)
enum Linked(val previous: Linked):
  case A extends Linked(null)
  case B extends Linked(Linked.A)
