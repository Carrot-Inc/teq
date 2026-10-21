// `Checking.checkAnnotArgs`' own tests of a binder's annotation (the review of `50e3a1b38`): `@targetName`'s argument a
// `Literal` of the typed tree (an ascription, a named one, an operation, a constant's name are none; parentheses are
// dropped), `@implicitNotFound`'s and `@implicitAmbiguous`'s of a constant type, widened and normalized (`ConstFold`'s
// type of an operation over constants; a block of a constant result; no fold of a string and a number), typed in the
// binder's context. scalac 3.8.4 gives these eight errors.
// expect: 21:31: error: @targetName needs a string literal as argument
// expect: 22:27: error: @targetName needs a string literal as argument
// expect: 23:28: error: @targetName needs a string literal as argument
// expect: 24:30: error: @targetName needs a string literal as argument
// expect: 26:27: error: target name cannot be empty
// expect: 27:36: error: @implicitNotFound requires constant expressions as a parameter
// expect: 32:33: error: @implicitNotFound requires constant expressions as a parameter
// expect: 33:55: error: @implicitAmbiguous requires constant expressions as a parameter
// absent: binder_annotation_args.scala:25:
// absent: binder_annotation_args.scala:28:
// absent: binder_annotation_args.scala:29:
// absent: binder_annotation_args.scala:30:
// absent: binder_annotation_args.scala:31:
import scala.annotation.{implicitAmbiguous, implicitNotFound, targetName}
final val m = "x"
type Ascribed = [@targetName(("x": String)) A] =>> List[A]
type Named = [@targetName(name = "x") A] =>> List[A]
type Concat = [@targetName("a" + "b") A] =>> List[A]
type Constant = [@targetName(m) A] =>> List[A]
type Parens = [@targetName(("x")) A] =>> List[A]
type Empty = [@targetName("") A] =>> List[A]
type Widened = [@implicitNotFound(("x": String)) A] =>> List[A]
type Block = [@implicitNotFound({ println("x"); "x" }) A] =>> List[A]
type Folded = [@implicitNotFound("a" + "b") A] =>> List[A]
type FoldedConstant = [@implicitNotFound(m + "y") A] =>> List[A]
type NamedConstant = [@implicitNotFound(msg = "x") A] =>> List[A]
type Mixed = [@implicitNotFound("a" + 1) A] =>> List[A]
def local(s: String) = { type L = [@implicitAmbiguous(s) A] =>> List[A]; 1 }
