// dotty's variance check of a class's members by their locality (typer/VarianceChecker.scala at 3.8.4): a private
// member read or assigned through a local of `this.type` is no longer local (the prefix is a TermRef, not the owner's
// `this`: isAccessPrivilegedThisType), a compound assignment through another instance marks the var as assigned
// non-locally (an Assign, Applications.scala's typedOpAssign), a case class's `private[this]` parameter is checked
// through the getter of `copy`'s default Desugar makes for it, and a trait parameter's private var through its setter.
class ReadSelf[-A](a: A):
  private val x: A = a
  def f(): Int = { val self: this.type = this; val y = self.x; 1 }
class WriteSelf[+A](a: A):
  private var x: A = a
  def f(v: A @scala.annotation.unchecked.uncheckedVariance): Int = { val self: this.type = this; self.x = v; 1 }
class CompoundSelf[+A](a: List[A]):
  private var x: List[A] = a
  def f(): Int = { val self: this.type = this; self.x ++= Nil; x.size }
class CompoundOther[+A](a: List[A]):
  private var x: List[A] = a
  def f(o: CompoundOther[A @scala.annotation.unchecked.uncheckedVariance]): Int = { o.x :+= o.x.head; x.size }
case class CopyDefault[-A](private[this] val a: A)
trait SetterParam[+A](private var x: A):
  def get: A = x
// expect: 7:15: error: contravariant type A occurs in covariant position in type A of value x
// expect: 10:15: error: covariant type A occurs in invariant position in type A of variable x
// expect: 13:15: error: covariant type A occurs in invariant position in type List[A] of variable x
// expect: 16:15: error: covariant type A occurs in invariant position in type List[A] of variable x
// expect: 18:49: error: contravariant type A occurs in covariant position in type [A²]: A of method copy$default$1
// expect+
// expect+ where:    A  is a type in class CopyDefault
// expect+           A² is a type variable
// expect: 19:35: error: covariant type A occurs in contravariant position in type A of parameter x_=
