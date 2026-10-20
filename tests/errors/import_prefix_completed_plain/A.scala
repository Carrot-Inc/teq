// `prefix` is a plain inline method, which the typer keeps as a call: as an import's prefix it is no pure
// expression (`Checking.checkLegalImportOrExportPath`'s `isIdempotentExpr`: a method that is no stable member),
// whatever the state of its signature.
// expect: Use.scala:3:10: error: import prefix is not a pure expression
// expect: 1 error found
object O:
  val x = 1
inline def prefix: O.type = O
