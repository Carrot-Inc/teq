// A by-name parameter of a dependent function type: `Parsers.funParamClause` reads no `=>` before a named
// parameter's type there, scalac 3.8.4's E040; teq parses it (master's). Met by the review of `cf6cad6c1`'s item 2.
// expect: 4:14: error: an identifier expected, but '=>' found
type T = (x: => Int) => Int
