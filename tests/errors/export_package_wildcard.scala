// A wildcard export of a package's members is an implementation restriction (`Checking.checkLegalExportPath`); the
// forwarders are made all the same.
// expect: error: Implementation restriction: package lib is not a valid prefix for a wildcard export, as it is a package
// expect: 1 error found
package lib:
  val v = 1
object E:
  export lib.*
@main def run(): Unit = println(E.v)
