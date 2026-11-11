package dgiv

// A given whose using clauses leave a parameter to its default, called on an object whose only
// initialiser is an alias given (a lazy val): touching the object runs no code, read from its
// source or from its pickle, so a downstream module's call places the arguments' temporaries the
// same way in a build over every module's sources and in one over the products.
class A
class C
class D
class R
object Givens:
  given Unit = ()
  given a(using Unit): A = { print("A"); new A }
  given d(using Unit): D = { print("D"); new D }
  given b(using a: A)(using c: C = { print("C"); new C }, d: D): R = { print("R"); new R }
  def use(using r: R): Unit = println("!")
