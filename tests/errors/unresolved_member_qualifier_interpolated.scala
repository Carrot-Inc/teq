// Two splices whose qualifier is a selection of no member: each `c.foo` errs, and the selection on it (`.bar`) is typed
// as an error with no symbol (Trees.scala:258-263), which the interpolation's stability test reads as no path. The
// two not-a-member errors are the whole output, as scalac 3.8.4's (6:28 and 6:41, 0-based).
// expect: 6:29: error: value foo is not a member of C
class C
def f(c: C): String = s"${c.foo.bar}/${c.baz.qux}"
// expect: 6:42: error: value baz is not a member of C
// expect: 2 errors found
