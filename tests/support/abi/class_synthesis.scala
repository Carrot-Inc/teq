// The class files of the members dotty synthesizes: a case class's by rule (`SyntheticMembers`), an
// object's constant vals with a final getter and no field (`Memoize`), an enum's stored `$values`
// and its lookups (`DesugarEnums`); a union of case classes erases to their `erasedLub` over the
// parents `Desugar.classDef` gives them (`Product`, `Serializable`).
// abi: C#toString C#hashCode C#equals C#canEqual C#productPrefix C#productArity C#productElement C#productElementName O$#K O$#L E$#$values E$#values E$#valueOf E$#fromOrdinal E#values E#valueOf E#fromOrdinal U$#cd U$#cn U$#sp U$#ec
package class_synthesis
case class C(x: Int, s: String)
object O:
  final val K = 7
  final val L: 9 = 9
enum E:
  case A, B
case class D(y: Int)
case object N
object U:
  def cd(x: C | D): Int = 0
  def cn(x: C | N.type): Int = 0
  def sp(x: Some[Int] | (Int, Int)): Int = 0
  def ec(x: E | C): Int = 0
