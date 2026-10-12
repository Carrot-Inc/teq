// expect: type mismatch: found 5, required Throwable
// expect: type mismatch: found "text", required Throwable
// expect: this pattern of type List[Any] can never match a value of type Throwable
// expect: A try without catch or finally is equivalent to putting its body in a block; no exceptions are handled.
// expect: Discarded non-Unit value of type Int. Add `: Unit` to discard silently.
// teq: --werror

def a(): Nothing = throw 5

def b(): Int = throw "text"

def c(): Int =
  try 1
  catch case List(x) => x

def d(): Int = try 2

def e(): Int =
  try 3
  finally 4
