// An inferred using argument naming the class's own using parameter accessor in its constructor's code is dotty's
// `Ident` (`typedImplicit`'s `tpd.ref`, `prefixIsElidable`), which `Constructors.intoConstr` reads as the
// constructor's parameter: what the superclass got (3), not the subclass's override (2).
def use(using x: Int): Int = x
class A()(using val s: Int):
  val saved = use
class B()(using override val s: Int) extends A()(using s + 1)

@main def run(): Unit = println(new B()(using 2).saved)
