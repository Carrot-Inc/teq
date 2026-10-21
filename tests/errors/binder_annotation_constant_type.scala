// `Checking.checkAnnotArgs` reads the typed argument's type, `widenTermRefExpr.normalized` a constant type, no cast
// stripped (`stripCast` is `Annotations.argumentConstant`'s, `@nowarn`'s): a cast to `String` is no constant, an
// operation over one folds not; a cast to a singleton and a block of a folded constant are (the review of `b23cfe2a3`:
// ImplicitCast, FoldCastOperand, ImplicitCastSingleton, ImplicitBlockFold). scalac 3.8.4 gives these two errors.
// expect: 9:49: error: @implicitNotFound requires constant expressions as a parameter
// expect: 10:49: error: @implicitNotFound requires constant expressions as a parameter
// absent: binder_annotation_constant_type.scala:11:
// absent: binder_annotation_constant_type.scala:12:
type Cast = [@scala.annotation.implicitNotFound("x".asInstanceOf[String]) A] =>> List[A]
type Fold = [@scala.annotation.implicitNotFound("a".asInstanceOf[String] + "b") A] =>> List[A]
type Single = [@scala.annotation.implicitNotFound(("x": String).asInstanceOf["x"]) A] =>> List[A]
type Block = [@scala.annotation.implicitNotFound({ println("side"); "a" + "b" }) A] =>> List[A]
