// A downstream of a/ as tests/modules builds it (tests/modules/givenclassouter, givenclassread). scalac 3.8.4 prints the
// .expected, and so does each of teq's runs. Under `--release` the whole build's bundle names the parameter of the
// given class's method `sized(using n: Int)` by the member table (`db(cd)`, the given class's field `n` renamed) where
// the bundle over a/'s products keeps it (`db(n)`); the two builds' tables of member names are the same (275 names in
// givenclassouter), so the parameter's naming, not the table, differs. tests/modules' bundle rows of givenclassouter
// and givenclassread, under `--release` alone. Seen beside products_release_names_completion's fix (push 15).
package gpb

@main def run(): Unit =
  println(gpa.Plain.inside)
