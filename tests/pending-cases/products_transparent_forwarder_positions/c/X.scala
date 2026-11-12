// A downstream of a/ and b/ as tests/modules builds
// it. `tfex`, a transparent method of a/, calls
// an extension method exported from a class value
// (`Fw.ex(8)(9)`); b/'s `B.b6` holds its expansion.
// scalac 3.8.4 prints the .expected. teq's pickles
// of b/ are to be ones scalac reads (tests/modules'
// readtasty-b; TreeChecker, Typer.assertPositioned).
// The whole build's classes are to be the split
// build's (classes, subset, check-lean-whole).
package n2c

@main def run(): Unit =
  println(n2b.B.b6)
  println(n2.tfex)
