// expect: 8:11: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 9:11: error: Cannot reduce `inline if` because its condition is not a constant value
// expect: 15:11: error: Cannot reduce `inline if` because its condition is not a constant value: true | b
// expect: 3 errors found
object Conditions:
  inline def cond: Int = inline if List(1).sum > 0 then 1 else 2
  inline def arg(inline x: Int): Int = inline if x > 0 then x else -x
  def g = cond
  def h = arg("ab".length)
  inline def pick(inline x: Int): Int = inline x match
    case 2 => 1
    case _ => 2
  def i = pick("ab".length)
  inline def strict(inline b: Boolean): Int = inline if true | b then 1 else 2
  def j = strict("ab".isEmpty)
  def k = strict(false)
