// A method of a subclass with the name, the parameters and as many type parameters as an
// inherited generic one, but other bounds, is an overload rather than an override: a call of
// the inherited one runs the inherited body, on the interpreter as on the other targets.
trait Parent:
  def f[A <: Number](x: A): String = "parent"
  def g[A <: CharSequence](x: A): String = "parent g"
class Child extends Parent:
  def f[B <: CharSequence](x: B): String = "other"
  override def g[B <: CharSequence](x: B): String = "child g"
@main def run(): Unit =
  val p: Parent = new Child
  println(p.f[Number](null))
  println(p.g[String]("s"))
  println(Child().f("s"))
