// A concrete given is final (dotty's `Parsers.givenDef`, an alias's; `Desugar`, a given class's def): overriding
// one is scalac's E164. teq accepted it, and the class failed to load once a trait's given getter in the class
// mixing it in became final, as scalac's (tests/classpath/jvm/jar_trait_field_kinds).
// expect: 9:18: error: given x cannot override final member given x in trait T
trait T:
  given x: Int = 1
class C extends T
class D extends C:
  override given x: Int = 3
@main def run(): Unit = println(D().x)
