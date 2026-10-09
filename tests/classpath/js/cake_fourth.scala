// jars: fixtures
// targets: js interp jvm
// Further chimney rules from the fixtures jar: an alias of an object nested in a
// trait read through another trait's self type, the constructor of a class nested in a trait
// extended from a trait whose self type derives from it, the singleton of an inner class's
// member through a pattern's value, a member selected on an `asInstanceOf` result, whose
// signature is seen through the cast's type, the outer instance of a trait nested in an object
// of the enclosing trait, a case class of a base trait matched through the self type, private
// members of a class nested in a trait read from its companion, a companion's own `unapply`
// over an abstract type, a local class's parameter shadowing a lambda's, and an override
// through a covariant parameter inherited at three instantiations, and the type variables of
// a case named in a nested match of its body, and `Predef.Map.Map1` as the std's small map.
import fix.cake.{FoBoth, FoConcrete, FoIgnore, FoImpl, FoInto, FoLeaf, FoMapped, FoMaps, FoPlain, FoReq, FoRunner}

@main def main(): Unit =
  val i = new FoImpl
  val p = i.pair(i.ExistentialType[Int](using "int"), i.ExistentialType[Long](using "long"))
  println(p.map((a, b) => i.show(a) + "," + i.show(b)))
  println(i.mk[Int](using "int"))
  println(i.viaMatch(Some(i.ExistentialType[Long](using "long"))))
  println(i.castK(i.ExistentialType[Int](using "int")))
  println(i.viaImpl(7)(using "seven"))
  println(i.names[Int, Long]("a", "b"))
  println(i.ctorOf[Int]("int"))
  println(i.wrapped(List(i.ExistentialType[Int](using "w1"), i.ExistentialType[Long](using "w2"))))
  val path = i.Path.select(i.Path.select(i.Path.Root, "a"), "b")
  println(path)
  println(path match { case i.Path.AtField(h, rest) => h + " " + rest; case _ => "root" })
  println(new FoInto(3, true).get)
  println(new FoInto("s", false).both)
  val backend: FoPlain[Option] = new FoConcrete
  println(backend.send(new FoReq[Int, Any](5)))
  val runner = new FoRunner
  println(runner.run(FoMapped(FoLeaf(2), (a: Int, n: Int) => a + n + 1)))
  println(runner.run(FoBoth(FoLeaf("x"), FoMapped(FoIgnore, (u: Unit, n: Int) => n))))
  println(FoMaps.single("k", 1).updated("j", 2).toList.sorted)
