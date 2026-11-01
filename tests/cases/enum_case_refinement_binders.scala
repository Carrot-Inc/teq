// `DesugarEnums.typeParamIsReferenced`'s `underBinders` subtracts names by namespace: a refinement's val or def named
// like the enum's type parameter hides nothing, so the case's parameter that names `T` takes the enum's `T`; a
// refinement's type member of that name hides it.
enum E1[T](val s: Int):
  case C(x: { val T: Int; type U = T }, override val s: Int) extends E1[Int](s)
enum E2[T](val s: Int):
  case C(x: { def T: T }, override val s: Int) extends E2[Int](s)
enum E3[T](val s: Int):
  case C(x: { type T = Int; val y: T }, override val s: Int) extends E3[Int](s)

@main def run(): Unit =
  val a: E1.C[String] = E1.C(new { val T: Int = 1; type U = String }, 4)
  val b: E2.C[String] = E2.C(new { def T: String = "t" }, 5)
  val c: E3.C = E3.C(new { type T = Int; val y: Int = 2 }, 6)
  println(s"${a.s} ${b.s} ${c.s}")
