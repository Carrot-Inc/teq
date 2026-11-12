// A member alias through a prefix is a reference kept until a
// reader expands it: substituting `F := C#K` in `F[T]` gives
// `AppliedType(TypeRef(C, K), T)`, since `appliedTo` reduces only a lambda
// itself or an eta-reduction, never an alias of one (TypeApplications.scala
// `appliedTo`). So the case `F[f]` reads `C#K[f]`, binds `f` from the
// scrutinee `C#K[Int]`, and `X` is `Int`, as scalac 3.8.4 runs it. The
// alias reference kept until dealias: moves with type-representation.
class C:
  type K[X] = Int
type M[F[_], T] = F[T] match
  case F[f] => f
type X = M[C#K, Int]
@main def run(): Unit =
  val x: X = 1
  println(x)
