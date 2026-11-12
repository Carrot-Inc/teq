// A non-final val of a constant type named by an identifier is its literal
// (FirstTransform.transformIdent's constToLiteral: the reference's type is
// constant and an identifier's refPurity is PurePath), so neither its file
// nor its object is initialised, and `z` inside P's initialiser reads 42.
package cin
val x2: 43 = { println("init x2"); 43 }
