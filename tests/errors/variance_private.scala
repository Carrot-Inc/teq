// A private member is local to `this` until a selection through another instance, or through a
// companion, makes it every instance's, which subjects it to the variance check (dotty's
// `PrivateLocal`: Namer's `checkFlags`, `isAccessibleFrom` resetting `Local`,
// VarianceChecker.relativeVariance); the alternatives of its name go with it, a private var
// assigned through another instance is invariant (`AssignedNonLocally`), and a qualified private
// member or a private class parameter is never local.
class Other[+A]:
  private def bad(x: A): A = x
  def pass[B >: A](other: Other[B], b: B): B = other.bad(b)
class Self[+A]:
  private def ok(x: A): A = x
  def pass(a: A @annotation.unchecked.uncheckedVariance): A = ok(a)
  def pass2(a: A @annotation.unchecked.uncheckedVariance): A = this.ok(a)
class Comp[+A]:
  private def bad(x: A): A = x
object Comp:
  def use[A](c: Comp[A], a: A): A = c.bad(a)
class Qual[+A]:
  private[Qual] def bad(x: A): A = x
class Field[-A](private val a: A)
class Var[+A](init: A):
  private var v: A = init
  def read[B >: A](o: Var[B]): B = o.v
class VarSet[+A](init: A):
  private var v: A = init
  def write[B >: A](o: VarSet[A]): Unit = o.v = v
class Over[+A]:
  private def bad(x: A): A = x
  private def bad(x: A, y: Int): A = x
  def pass[B >: A](other: Over[B], b: B): B = other.bad(b, 1)
class Unused[+A]:
  private def hidden(x: A): A = x
// expect: 8:19: error: covariant type A occurs in contravariant position in type A of parameter x
// expect: 15:19: error: covariant type A occurs in contravariant position in type A of parameter x
// expect: 19:25: error: covariant type A occurs in contravariant position in type A of parameter x
// expect: 20:29: error: contravariant type A occurs in covariant position in type A of value a
// expect: 25:15: error: covariant type A occurs in invariant position in type A of variable v
// expect: 26:21: error: covariant type A occurs in contravariant position in type VarSet[A] of parameter o
// expect: 28:19: error: covariant type A occurs in contravariant position in type A of parameter x
// expect: 29:19: error: covariant type A occurs in contravariant position in type A of parameter x
// expect: Implementation limitation: private val parameters cannot be inferred to be local
// absent: 11:
// absent: 22:
// absent: 31:
// Through `C.this` from a class nested in `C`, a private var stays local: no other instance.
class OuterVar[+A](init: A):
  private var v: A = init
  def get: A = v
  class Inner:
    v = init
    def reset(): Unit = OuterVar.this.v = init
// absent: 47:
