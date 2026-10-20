// An import or export clause is a qualifier, a `.` and its selectors (`Parsers.importExpr`): a name without its
// `.` is scalac's syntax error at the token after it, the name the qualifier and the selector the error name
// (`namedSelector(termIdent())`), which binds nothing; a `*`, a brace or a `given` where the qualifier should be
// is "an identifier expected", the qualifier the error name, which resolves to nothing. The typer runs on the
// clauses as scalac's does (`TyperPhase.isRunnable`): the qualifier of each is checked. The comments keep the
// syntax error's token off the qualifier, where scalac's reporter would hide the typer's message at one place;
// teq shows the purity message beside the stability one, which scalac's reporter hides at the same place.
// expect: 20:1: error: expected '.', found new line
// expect: 21:1: error: expected '.', found new line
// expect: 20:8: error: cannot resolve import: * not found
// expect: 21:8: error: expected an identifier, found '{'
// expect: 22:8: error: expected an identifier, found 'given'
// expect: 24:1: error: expected '.', found new line
// expect: 23:8: error: (f : => Int) is not a valid export prefix, since it is not an immutable path
// expect: 23:8: error: import prefix is not a pure expression
// expect: 8 errors found
object O
def f = 1
import O // the qualifier is the object
import * // `*` is an identifier here, found nowhere
import {O} // the selectors of no qualifier
import given // a wildcard of no qualifier
export f // a parameterless method, no path
@main def run(): Unit = println(f)
