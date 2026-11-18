// expect: 9:14: error: too many type arguments
// expect: 10:14: error: this value does not take type arguments
// expect: 11:14: error: too many type arguments
// A type argument list after an expression written in braces, in parentheses around them too, applies to the
// block's value and never reaches a method the block names. scalac: "method apply in class StringOps does not
// take type parameters" (the first and the third), "method apply in trait Function1 does not take type parameters".
def mk[T]: String = "mk"
def id[T](x: T): T = x
def t: Any = { mk }[Int]
def u: Any = { id }[Int](1)
def v: Any = ({ mk })[Int]
