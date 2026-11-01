// jars: callable-lib scala-library
// targets: js interp jvm
// A function-typed class (its instances JavaScript functions) over a library class whose val is named like a
// function's own `name` or `length`: the library's value where nothing overrides it, the class's (or its subclass's)
// where an `override val` does, as scalac prints them. The function's own property goes before the class's fields are
// written, and the method a superclass names so is copied without running a val's getter (`$callable`).
class C extends callable.Base with (Int => Int):
  override val name = "c"
  def apply(i: Int) = i + name.length
class D extends callable.Base with (Int => Int):
  def apply(i: Int) = i
class D2 extends D:
  override val name = "d2"
class P extends callable.Base:
  override val name = "p"
class F extends callable.BaseL with (Int => Int):
  def apply(i: Int) = i + length

@main def run(): Unit =
  println(s"${P().name} ${P().show}")
  println(s"${D().name} ${D().show}")
  val c = C()
  println(s"${c.name} ${c.show} ${c(1)}")
  val f = F()
  println(s"${f.length} ${f(1)}")
  val d2 = D2()
  println(s"${d2.name} ${d2.show}")
