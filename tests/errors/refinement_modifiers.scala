// A refinement's members are declarations without modifiers (dotty's `refineStatSeq`, `checkLegal`),
// in braces wherever a type starts with them.
trait A
trait B
type T = {given x(using a: A): B}
// expect: 5:17: error: refinement cannot be `given`
