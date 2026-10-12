// An inferred type widens a union as dotty's `widenInferred` does: a join the compiler made
// (soft) to its members' join, `if c then 1 else 2` an `Int`, `if c then 1 else "one"` an `Int |
// String`; a written (hard) union by its members, kept a union, `1: (1 | 2)` an `Int`; an alias
// of a union kept by name (`widen` does not dealias it), so `1: MonthNumber` is a `MonthNumber`
// that a `MonthNumber` accepts (pos/22219), and a join kept where a union is expected.
type MonthNumber = 1 | 2
type Answer = 1 | "one"
def pick(c: Boolean) = if c then 1 else 2
def mixed(c: Boolean) = if c then 1 else "one"
def written(c: Boolean) = (if c then 1 else "one"): 1 | "one"
def aliased(c: Boolean) = (if c then 1 else "one"): Answer
@main def run() =
  val x = 1: MonthNumber
  val y: MonthNumber = x
  val w = 2: (1 | 2)
  val n: Int = w + 1
  val p: Int = pick(true) + 1
  val m: Int | String = mixed(false)
  val h: Int | String = written(true)
  val a: Answer = aliased(false)
  val l: List[1 | 2] = List(if p > 1 then 1 else 2)
  println((y, n, p, m, h, a, l))
