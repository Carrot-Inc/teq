// expect: 20:10: warning: the type test for Types.this.Type[A$1] cannot be checked at runtime because it refers to an abstract type member or type parameter
// expect: 25:10: warning: the type test for Types.this.Type[A$1] cannot be checked at runtime because it refers to an abstract type member or type parameter
// expect: 32:10: warning: the type test for Plain.this.T cannot be checked at runtime because it refers to an abstract type member or type parameter
// expect: 50:10: warning: the type test for Variances.this.Co[Any] cannot be checked at runtime because it refers to an abstract type member or type parameter
// expect: 53:10: warning: the type test for Variances.this.Contra[Nothing] cannot be checked at runtime because it refers to an abstract type member or type parameter
// absent: 23:10: warning
// absent: 39:10: warning
// absent: 50:20: error
// absent: 53:24: error
// teq: --werror
// An extractor whose input is an abstract type tests a scrutinee of a class against it at its
// parameters' maximised types (`Co[Any]`, `Contra[Nothing]`, at an invariant position a fresh
// `A$1`), a test that passes any value, as scalac 3.8.4 warns (E092), unless the parameter is
// written `T @unchecked`; over a scrutinee that conforms there is no test.
trait Types:
  type Type[A]
  object Ex:
    def unapply[A](a: Type[A]): Some[Int] = Some(1)
  def any(x: Any): Int = x match
    case Ex(n) => n
    case _ => 0
  def conforming(x: Type[String]): Int = x match
    case Ex(n) => n
  def option(x: Option[Int]): Int = x match
    case Ex(n) => n
    case _ => 0
trait Plain:
  type T
  object Ex2:
    def unapply(a: T): Some[Int] = Some(1)
  def any(x: Any): Int = x match
    case Ex2(n) => n
    case _ => 0
trait Marked:
  type T
  object Ex3:
    def unapply(a: T @unchecked): Some[Int] = Some(1)
  def any(x: Any): Int = x match
    case Ex3(n) => n
    case _ => 0
trait Variances:
  type Co[+A]
  type Contra[-A]
  class Box[A]
  object ExCo:
    def unapply[A](x: Co[A]): Some[Box[A]] = ???
  object ExContra:
    def unapply[A](x: Contra[A]): Some[A] = ???
  def co(x: Any): Box[Any] = x match
    case ExCo(v) => v
    case _ => new Box[Any]
  def contra(x: Any): Nothing = x match
    case ExContra(v) => v
    case _ => throw new Exception
