// An enum case takes its enum's type parameters where its parameters' types or its parents name one
// (`DesugarEnums.typeParamIsReferenced`): in a refinement's member, under a polymorphic function type (no binder of
// the search's), but not under a refinement member of that name; a value case of a covariant enum extends the enum
// applied to the parameter's lower bound (`interpolatedEnumParent`).
enum E[T]:
  case C(x: AnyRef { def m: T }) extends E[Int]
  case P(f: [T] => T => T) extends E[Int]
  case H(x: AnyRef { type T = Int }) extends E[Int]

enum B[+T >: Null <: AnyRef]:
  case K

@main def run(): Unit =
  val c: E.C[String] = E.C(new AnyRef { def m: String = "m" })
  val p: E.P[Int] = E.P([X] => (x: X) => x)
  val h: E.H = E.H(new AnyRef { type T = Int })
  val k: B[String] = B.K
  println(c.ordinal)
  println(p.f(3))
  println(h.ordinal)
  println(k)
