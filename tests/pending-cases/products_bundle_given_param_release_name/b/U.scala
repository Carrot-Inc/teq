// A downstream of a/ as tests/modules builds it (tests/modules/givenclassouter,
// givenclassread). scalac 3.8.4 prints the .expected. Under
// `--release` teq's bundle of the whole build and the bundle over
// a/'s products are to name the parameter of the given class's
// method `sized(using n: Int)` alike (tests/modules' bundle rows
// of givenclassouter and givenclassread, under `--release` alone).
package gpb

@main def run(): Unit =
  println(gpa.Plain.inside)
