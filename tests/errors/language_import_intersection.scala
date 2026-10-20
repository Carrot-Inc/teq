// A language import is checked by the classes of the path's type (`Checking.checkLegalImportPath`:
// `path.tpe.classSymbols`), an intersection's united: an alias typed `AnyRef & scala.language.type`
// is still a reference to the language object.
// expect: 6:76: error: no aliases can be used to refer to a language import
// expect: 1 error found
object Test { val l: AnyRef & scala.language.type = scala.language; import l.strictEquality }
