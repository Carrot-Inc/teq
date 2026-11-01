// A subclass's methods and fields spelled like its base's storage (A's copy of `s`, a property of the instance): the
// copy is allocated against its class's members and its bases', and against the members of every class extending A,
// two levels down included (`Constructors.checkNoFieldClashes`), whose members keep their names: a subclass's `$s`
// beside its own `$$s`, a descendant's `$$$s`, an override, a base's private member.
class A(val s: Int):
  val saved = s
  private def $$$$s: Int = 7
  def viaPrivate = $$$$s
class B(override val s: Int) extends A(s + 1):
  def $$s: Int = 42
  def $s: Int = 41
class C(override val s: Int) extends A(s + 1):
  val $$s: Int = 43
class D(override val $$s: Int) extends C(7)
class E(s0: Int) extends B(s0):
  def $$$s: Int = 50
  override def $s: Int = 51
class F(s0: Int) extends B(s0):
  def $$$$$s: Int = 60
@main def run(): Unit =
  val b = B(2)
  println(s"B ${b.saved} ${b.$$s} ${b.$s} ${b.viaPrivate}")
  val c = C(2)
  println(s"C ${c.saved} ${c.$$s}")
  val d = D(9)
  println(s"D ${d.saved} ${d.$$s} ${d.s}")
  val e = E(2)
  println(s"E ${e.saved} ${e.$$s} ${e.$s} ${e.$$$s} ${e.viaPrivate}")
  val f = F(2)
  println(s"F ${f.saved} ${f.$$s} ${f.$s} ${f.$$$$$s} ${f.viaPrivate}")
  val bb: B = e
  println(s"BB ${bb.$s} ${bb.$$s}")
