// An inferred type widens a written (hard) union by its members (dotty's `widenInferred`, its
// `widenUnion`: each member widened, the union kept hard), so `h`, a result declared `1 | "one"`
// read through `g`, and `m` are an `Int | String`, an `Int | String` and an `Int`; an alias of the
// union is kept by name (`widenUnion` does not dealias it), so `h2` is the alias.
type U = 1 | "one"
def h(b: Boolean) = (if b then 1 else "one"): 1 | "one"
def h2(b: Boolean) = (if b then 1 else "one"): U
def h3(b: Boolean): 1 | "one" = if b then 1 else "one"
def g = h3(true)
val m = 1: (1 | 2)
def run() =
  val a: Int = h(true)
  val b: 1 | "one" = h2(true)
  val c: 1 | "one" = g
  val d: Int = m
  val e: 1 | 2 = m
// expect: 12:16: error: type mismatch: found Int | String, required Int
// expect: 14:22: error: type mismatch: found Int | String, required 1 | "one"
// expect: 16:18: error: type mismatch: found Int, required 1 | 2
