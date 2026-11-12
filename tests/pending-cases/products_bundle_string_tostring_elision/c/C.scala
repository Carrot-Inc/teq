// Downstreams of a/ as tests/modules
// builds them. scalac 3.8.4 prints
// the .expected. b/'s `bw` holds the
// expansion of a/'s transparent `w`,
// `"" + y.toString` with the String
// argument "z". teq's Scala.js bundle
// of the whole program and the bundle
// built over a/'s and b/'s lean products
// are to write the `toString` of that
// string alike (tests/modules' bundle:
// one file, --split and --release alike).
package s3c

@main def run(): Unit =
  println(s3b.B.bw)
