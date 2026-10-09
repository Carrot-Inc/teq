// expect: 13:7: error: class D has conflicting base types A[Int] and A[String]
// expect: 19:7: error: class X inherits conflicting members: g in trait P and g in trait Q; declare an override in class X
// expect: 20:8: error: object O inherits conflicting members: g in trait P and g in trait Q; declare an override in object O
// expect: 23:7: error: trait R inherits conflicting members: g in trait P and g in trait Q; declare an override in trait R
// expect: 27:7: error: g in trait S cannot override the concrete g in trait Q without a member that both override
// expect: 5 errors found
trait A[T]:
  def f: T
trait B1 extends A[Int]:
  def f: Int = 1
trait B2 extends A[String]:
  def f: String = "s"
class D extends B1, B2

trait P:
  def g = "P"
trait Q:
  def g = "Q"
class X extends P with Q
object O extends P, Q
class Fine extends P, Q:
  override def g = "Fine"
trait R extends P, Q
class FromR extends R
trait S extends P:
  override def g = "S"
class Accidental extends Q with S
class Legitimate extends P with S

trait Abs:
  def k: String
trait Conc:
  def k = "C"
class K1 extends Abs with Conc
class K2 extends Conc with Abs
