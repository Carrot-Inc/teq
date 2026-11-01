// The property behind an accessor (dotty's field of a getter) is allocated apart from every member of its class and
// of the classes it extends, and from the runtime's (`Constructors.checkNoFieldClashes`): a user's `$$name` beside the
// field of `name`, a base's `$label` beside the field of `label`, an overriding accessor's field beside the one it
// overrides, and a case class's field of `name` beside the `$name` the runtime reads as an enum value's.
trait Named:
  def name: String
  def label: String
class C(val name: String, val $$name: String) extends Named:
  def label = "c"
class Base:
  val $label = "base"
class D(val name: String, val label: String) extends Base, Named
trait T:
  def tag: String
class A(val tag: String) extends T:
  def show = tag
  val saved = tag
class B(override val tag: String) extends A(tag + "!")
case class P(name: String, label: String) extends Named

@main def run(): Unit =
  val c = C("first", "second")
  println(s"${c.name} ${c.$$name}")
  val d = D("n", "l")
  println(s"${d.name} ${d.label} ${d.$label}")
  val b = B("x")
  println(s"${b.tag} ${b.saved} ${b.show} ${(b: A).tag}")
  val p = P("pn", "pl")
  println(s"$p ${p.productPrefix} ${p == P("pn", "pl")} ${p.name} ${p.label}")
