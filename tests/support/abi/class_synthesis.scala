// The class files of the members dotty synthesizes: a case class's by rule (`SyntheticMembers`), an
// object's constant vals with a final getter and no field (`Memoize`), an enum's stored `$values`
// and its lookups (`DesugarEnums`).
// abi: C#toString C#hashCode C#equals C#canEqual C#productPrefix C#productArity C#productElement C#productElementName O$#K O$#L E$#$values E$#values E$#valueOf E$#fromOrdinal E#values E#valueOf E#fromOrdinal
package class_synthesis
case class C(x: Int, s: String)
object O:
  final val K = 7
  final val L: 9 = 9
enum E:
  case A, B
