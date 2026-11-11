// A downstream of a/ and b/ as tests/modules builds it. `tfex`, a transparent method of a/, calls an extension method
// exported from a class value (`Fw.ex(8)(9)`); b/'s `B.b6` holds its expansion. scalac 3.8.4 prints the .expected, and
// so does each of teq's runs. scalac reading teq's pickles of b/ (tests/modules' readtasty-b) fails: "assertion failed:
// position not set for n2.Fw.ex # -1 of class Trees$Select in b/M.scala" (TreeChecker, Typer.assertPositioned). In
// reviewer #13's reading of the pickle, the selections the expansion keeps carry no position, and their qualifiers
// carry another source's. With an inline default left out (`Fw.ex(8)()` over `inline y: Int = ...`, reviewer #13's n1)
// teq failed earlier, on the getter's name, until 6d7b3f921a, and fails here since. The whole build's classes also
// differ from the split build's (classes, subset, check-lean-whole). Found by reviewer #13 of the workers exports
// helper (push 8); the same on integration b602956824.
package n2c

@main def run(): Unit =
  println(n2b.B.b6)
  println(n2.tfex)
