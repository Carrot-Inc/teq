// Downstreams of a/ as tests/modules builds them. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// b/'s `bw` holds the expansion of a/'s transparent `w`, `"" + y.toString` with the String argument "z". teq's
// Scala.js bundle of the whole program writes the `toString` of that string as the string, `"" + ("z")`, eliding the
// call on a string by construction (`Program::is_non_null_string`, a literal). The bundle built over a/'s and b/'s
// lean products, which reads `bw` back from b/'s pickle, calls it, `"" + $toStr("z")`, and carries the helper
// (tests/modules' bundle: one file, --split and --release alike). The receiver read back is no bare literal. The
// same on integration a097de53d9. tests/modules/overloadexpand's bundle row shows it since its JS products build
// completes (`Fmt.show[String]("beta", 2, "z")`, the transparent `toString` fix of the workers exports helper's push
// 10). Its classes, subset and check-lean-whole rows fail on the INLINED layers of
// products_transparent_expansion_inlined_layers and products_inlined_argument_marker: with the INLINED nodes set
// aside, b/'s whole and split pickles are identical.
package s3c

@main def run(): Unit =
  println(s3b.B.bw)
