// expect: 8:26: error: type mismatch: found 5, required Throwable
// expect: 10:22: error: type mismatch: found "text", required Throwable
// expect: 14:25: error: type mismatch: found Any, required Int
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
