// A `super` reference inside a class an inline body defines is that class's own (scalac's
// `typedSuper` allows it through a local definition), copied with the class at each expansion. A
// `super` of the inline method's own class is scalac's E082 at the definition ("Super call not
// allowed in inlineable method", `tests/errors/inline_definition_restrictions`). scalac prints the
// lines of the .expected file.
class Base:
  def hello: String = "base"
class Sub extends Base:
  override def hello: String = "sub"
  inline def viaLocal(tag: String): String =
    class L extends Base:
      override def hello: String = tag + ":" + super.hello
    new L().hello + " " + new Base { override def hello = tag + super.hello }.hello
@main def run(): Unit =
  println(Sub().viaLocal("L"))
  println(Sub().viaLocal("M"))
