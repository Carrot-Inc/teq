// jars: fixtures fixtures-shadow
// A member only the second copy of a class declares is not a member of the class read.
// expect: value onlySecond is not a member of Shadowed
import fix.shadow.Shadowed

val n = Shadowed().onlySecond
