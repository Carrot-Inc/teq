/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "java.util"
]
*/
package fix

// An entry may name a package: `java.util` is one, `scala.util` shares its last name and is none.
object PackageEntry:
  val attempt = scala.util.Try(1)
  val date = new java.util.Date() // assert: ForbiddenSymbol
