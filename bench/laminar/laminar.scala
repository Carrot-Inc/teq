//> using file ../../tools/script
//> using file ../reference/reference.scala
//> using file ../reference/scalajs.scala
// Laminar (github.com/raquo/Laminar), the UI library over Airstream, as one of teq's public reference projects for
// the JavaScript target: its pinned release checked out, built and tested under scalac 3.8.4 by its own sbt build
// (the oracle), exported with sbt-teq through the overlay, then compiled and tested by teq against the oracle's
// results (bench/reference/scalajs.scala has the verbs), over Airstream's released jar as its build resolves it.
// Run from the repository's root:
//
//   ./teq interp bench/laminar/laminar.scala -- <checkout|oracle|export|compile|test> <checkout> <out> [options]
//
// The overlay declares the DOM definitions build.sbt's `onLoad` writes as the command teq runs (domdefs.scala
// over the build's own project/DomDefsGenerator.scala), writing the checked-in files it writes; `laminar` is the
// project teq builds, the documentation's websiteJS and website left to sbt.
@main def laminar(argv: String*): Unit = Script.run {
  ScalaJsReference.main("laminar", argv)()
}
