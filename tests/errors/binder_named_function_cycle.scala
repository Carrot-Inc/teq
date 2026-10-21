// A named function type `(x: A) => Int` in a binder's annotation while the binder completes (the review of `cf6cad6c1`,
// item 2): `typedFunctionType.typedDependent`'s `apply` over the parameters, each one's `valOrDefDefSig`
// (`checkSimpleKinded` → `hasSimpleKind` → `hasAnyKind` → `translucentSuperType`) reading the binder: E046 at the
// parameter, as scalac 3.8.4 gives it (one named function type a file: scalac's reporter hides a second file's).
// expect: 7:34: error: Cyclic reference involving type A
class ann[T] extends scala.annotation.StaticAnnotation
type NamedFunctionCycle = [@ann[(x: A) => Int] A] =>> Int
