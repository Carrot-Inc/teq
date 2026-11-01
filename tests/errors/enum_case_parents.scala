// expect: 17:3: error: explicit extends clause needed because both enum case and enum class have type parameters
// expect: 19:8: error: enum case does not extend its enum class E2
// expect: 21:8: error: enum case does not extend its enum class E3
// expect: 23:16: error: illegal reference to type parameter T from enum case
// expect: 25:21: error: illegal reference to type parameter T from enum case
// expect: 27:8: error: cannot determine type argument for enum parent class E6,
// expect: 31:10: error: this type does not take type arguments
// expect: 33:3: error: explicit extends clause needed because both enum case and enum class have type parameters
// An enum case's parent as `Desugar.classDef` and `DesugarEnums` make it: a case with type parameters of its own
// writes its parent (`enumClassTypeRef`'s `TypedCaseDoesNotExplicitlyExtendTypedEnum`, at the case's start); a parent
// written that is not the enum (`Checking.checkEnumParent`); a reference to the enum's type parameter from a case
// with its own, or from a value case (`typeParamIsReferenced`); a value case of an invariant enum
// (`interpolatedEnumParent`); a type lambda's parameter hides the enum's of its name, so the case is not generic; an
// annotated case's tree starts at its annotation.
trait X
enum E1[T]:
  case C[U](x: U)
enum E2:
  case C(x: Int) extends X
enum E3[+T]:
  case K extends X
enum E4[T]:
  case C[U](x: T) extends E4[U]
enum E5[T]:
  case K extends E5[T]
enum E6[T]:
  case K
enum E7[T]:
  case C(f: ([T] =>> List[T])[Int]) extends E7[Int]
object Use:
  val c: E7.C[String] = E7.C(List(1))
enum E8[T]:
  @deprecated case C[U](x: U)
// A case with fewer type parameters than its enum: `enumClassTypeRef`'s parent, the enum applied to an `Any` per type
// parameter of the case, is too few for the enum and no class type, which `checkEnumParent` reports.
enum E9[A, B]:
  case C[T](x: T)
// expect: 37:3: error: explicit extends clause needed because both enum case and enum class have type parameters
// expect: 37:8: error: enum case does not extend its enum class E9
