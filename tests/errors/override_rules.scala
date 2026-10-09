// expect: 28:16: error: error overriding method f in trait T of type Int: method f of type (): Int has incompatible type
// expect: 30:16: error: error overriding method g in trait T of type (): Int: method g of type Int has incompatible type
// expect: 32:7: error: method v needs to be a stable, immutable value to override value v in trait S
// expect: 34:16: error: value lz must be declared lazy to override lazy value lz in trait T
// expect: 36:16: error: method fin cannot override final member method fin in trait T
// C6 overloads h, which is no error.
// expect: 40:16: error: method m overrides nothing
// expect: 42:16: error: error overriding method h in trait T of type (x: Int): Int: method h of type (x: Int): Any has incompatible type
// expect: 44:7: error: method toString needs `override` modifier to override the toString of Any
// expect: 48:7: error: error overriding method box in trait B of type Box[Int]: method box of type Box[Any] has incompatible type
// expect: 51:15: error: error overriding value f in trait U of type Int: value f of type String has incompatible type
// expect: 56:7: error: error overriding method g in trait V of type Int: method g in trait W of type String has incompatible type
// expect: 11 errors found
trait T:
  def f: Int = 1
  def g(): Int = 2
  lazy val lz: Int = 4
  final def fin: Int = 6
  def h(x: Int): Int = x
  def k[A](a: A): A = a
  def m(x: Int)(y: Int): Int = x
trait S:
  val v: Int
class Box[A](val a: A)
trait B:
  def box: Box[Int]
class C1 extends T:
  override def f(): Int = 1
class C2 extends T:
  override def g: Int = 2
class C3 extends S:
  def v: Int = 3
class C4 extends T:
  override val lz: Int = 4
class C5 extends T:
  override def fin: Int = 6
class C6 extends T:
  def h(x: String): Int = 1
class C7 extends T:
  override def m(x: Int, y: Int): Int = x
class C8 extends T:
  override def h(x: Int): Any = 1
class C9 extends T:
  def toString: String = "x"
class C10 extends T:
  override def k[B](a: B): B = a
class C11 extends B:
  def box: Box[Any] = Box(1)
trait U:
  val f: Int
class C12(val f: String) extends U
trait V:
  def g: Int
trait W:
  def g: String = ""
class C13 extends V, W
@main def run(): Unit = println(C10().f)
