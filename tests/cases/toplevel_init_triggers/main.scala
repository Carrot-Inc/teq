// Which accesses of a file's top-level definitions initialise its eager vals first, as an access
// of scalac's `<file>$package` object does: each file's initialiser prints, and the files that a
// shape does not initialise print nothing.
@main def main(): Unit =
  println("def"); println(tdef.f(1))
  println("extension")
  locally:
    import text.*
    println(2.plus1)
  println("lazy val"); println(tlazy.lz)
  println("alias given")
  locally:
    import talias.given
    println(summon[Ordering[Int]].compare(1, 2))
  println("param given")
  locally:
    import tpg.given
    println(summon[Ordering[List[Int]]].compare(List(1), List(2)))
  println("literal val"); println(tval.x)
  println("final typed val"); println(tft.x)
  println("var read"); println(tvarr.v)
  println("var write"); tvarw.v = 5
  println("inline reading eager"); println(tinl.readE)
  println("inline const"); println(ninl.k)
  println("type alias")
  locally:
    val t: nalias.T = 3
    println(t)
  println("class"); println(new nclass.C().m)
  println("object"); println(nclass.O.m)
  println("nested object"); println(nnest.Outer.Inner.x)
  println("end")
