// A cast's type erases as dotty's erasure erases it, through bounds, applied and composite types,
// each class by its own erasure (TypeErasure.apply, Definitions.specialErasure): `Tuple` to
// `Product`, `Singleton` to `Object`, a context function to its function, a function past 22
// parameters to `FunctionXXL`, a `*:` chain by its arity (`erasePair`, `tupleArity`): the tuple
// class of a known arity, `TupleXXL` past 22, `Product` where it ends in the alias `EmptyTuple`.
// A type test of an abstract type erases its bound the same way.
case class P(n: Int)
def attempt(label: String)(f: => Any): Unit =
  try { f; println(label + " ok") }
  catch case _: ClassCastException => println(label + " CCE")
def tupleBound[T <: Tuple](x: Any): Unit = { x.asInstanceOf[T]; () }
def singletonBound[T <: Singleton](x: Any): Unit = { x.asInstanceOf[T]; () }
def contextBound[T <: (Int ?=> Int)](x: Any): Unit = { x.asInstanceOf[T]; () }
def nestedBound[U <: Tuple, T <: U](x: Any): Unit = { x.asInstanceOf[T]; () }
def tupleUnion(x: Any): Unit = { x.asInstanceOf[Tuple | Product]; () }
def tupleIntersection(x: Any): Unit = { x.asInstanceOf[Tuple & Product]; () }
def tested[T <: Tuple](x: Any): Boolean = x match
  case _: T @unchecked => true
  case _ => false
type Big = Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: EmptyTuple.type
@main def run(): Unit =
  val p: Any = P(3)
  val s: Any = "abc"
  attempt("direct Tuple") { p.asInstanceOf[Tuple]; () }
  attempt("bound Tuple") { tupleBound[(Int, Int)](p) }
  attempt("bound Tuple, a string") { tupleBound[(Int, Int)](s) }
  attempt("nested bound Tuple") { nestedBound[Tuple, (Int, Int)](p) }
  attempt("union Tuple") { tupleUnion(p) }
  attempt("intersection Tuple") { tupleIntersection(p) }
  attempt("bound Singleton") { singletonBound[s.type](s) }
  val f: Any = (x: Int) => x
  attempt("bound context") { contextBound[Int ?=> Int](f) }
  attempt("bound context, a string") { contextBound[Int ?=> Int](s) }
  attempt("spelled cons") { ((1, 2, 3): Any).asInstanceOf[Int *: Int *: EmptyTuple]; () }
  attempt("spelled cons, a string") { s.asInstanceOf[Int *: Int *: EmptyTuple]; () }
  attempt("cons of a tuple") { ((1, 2, 3): Any).asInstanceOf[Int *: (Int, Int)]; () }
  attempt("cons of a tuple, a pair") { ((1, 2): Any).asInstanceOf[Int *: (Int, Int)]; () }
  attempt("cons of EmptyTuple.type") { ((1, 2): Any).asInstanceOf[Int *: Int *: EmptyTuple.type]; () }
  attempt("cons of EmptyTuple.type, a triple") { ((1, 2, 3): Any).asInstanceOf[Int *: Int *: EmptyTuple.type]; () }
  println(tested[(Int, Int)](p))
  println(tested[(Int, Int)](s))
  val pair: Any = (1, 2)
  val big: Any = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)
  attempt("pair as Big") { pair.asInstanceOf[Big]; () }
  attempt("23 as Big") { big.asInstanceOf[Big]; () }
  println(big.asInstanceOf[Product].productElement(22))
  val f24: Any = (a0: Int, a1: Int, a2: Int, a3: Int, a4: Int, a5: Int, a6: Int, a7: Int, a8: Int, a9: Int, a10: Int, a11: Int, a12: Int, a13: Int, a14: Int, a15: Int, a16: Int, a17: Int, a18: Int, a19: Int, a20: Int, a21: Int, a22: Int, a23: Int) => a0
  attempt("24 as 23") { f24.asInstanceOf[(Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) => Int]; () }
