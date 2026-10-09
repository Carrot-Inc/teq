// expect: 15:30: error: Deferred inline method apply in trait Inliner cannot be invoked
// expect: 17:29: error: Cannot reduce `inline if` because its condition is not a constant value: b
// expect: 2 errors found
// An error of an inline expansion is reported where the inlined code stands, which scalac 3.8.4's
// reporter shows at the outermost call with the inline stack trace and keeps once per position
// (`UniqueMessagePositions`, over the inlined position): the second expansion of `wrap` and of
// `cond` fails at the place of the first one's error and is not reported again; an error placed
// at the call (`compiletime.error`) is each call's own.
trait Inliner:
  inline def apply(x: Int): Int

inline def wrap(f: Inliner): Int = f(3)
inline def cond(b: Boolean): Int = inline if b then 1 else 2

def three(f: Inliner): Int = wrap(f)
def four(f: Inliner): Int = wrap(f)
def five(b: Boolean): Int = cond(b)
def six(b: Boolean): Int = cond(b)
