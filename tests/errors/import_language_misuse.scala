// `Checking.checkLegalImportPath`: an import that looks like a language import (`TreeInfo.languageImport`, by its
// written path) must refer to `scala.language`, and no alias may refer to it.
// expect: 8:22: error: import looks like a language import, but refers to something else: object language
// expect: 9:46: error: no aliases can be used to refer to a language import
// expect: 10:55: error: no aliases can be used to refer to a language import
// expect: 3 errors found
object language { object strictEquality }
object Use1 { import language.strictEquality }
object Use2 { val l = scala.language; import l.strictEquality }
object Use4 { import scala.{language as lang}; import lang.implicitConversions }
