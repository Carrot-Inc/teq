// jars: fixtures
// targets: js interp jvm
// More of chimney's rules from the fixtures jar: an existential's `Underlying` read
// through a value and seen from the enclosing cake, a protected member reached from an object
// nested in a self-typed trait, an alias the self type's member instantiates, an extractor
// object nested in an implicit class, a case class nested in an inner object under a type test,
// a parent class nested beside the object extending it, `new Ty.Cache` taking its prefix as the
// enclosing instance, and a package's extension exported from another package.
import fix.cake.{ExImpl, PatImpl, SupImpl}
import fix.pext.dsl.*

@main def main(): Unit =
  val e = new ExImpl
  println(e.go(e.make))
  println(e.Inner.run(e.make))
  val p = new PatImpl
  println(p.read("on=yes"))
  println(p.read("off"))
  println(p.fresh(Some("x")))
  println(p.fresh(None))
  val s = new SupImpl
  println(s.Lits.One.show)
  println(s.Two.show)
  println(s.cached)
  println("hey".shout)
