// A scalac 3.8.4 quirk the corpus works around (the amount type is nested in an object): through
// an `export` alias of a top-level opaque type, the results of the companion's methods are seen
// as the underlying type. scalac accepts `leak` and `leak2` and rejects `t` (`value isNegative is
// not a member of p.amount.Amount`); with `import p.amount.Amount` in the place of the alias all
// three behave as the opaque type. teq keeps the type opaque through the alias: it accepts `t`
// and rejects `leak` and `leak2` (`type mismatch: found Amount, required Long`).
package q
import p.*
object Main:
  def t(a: Amount, b: Amount): Boolean = (a - b).isNegative
  def leak(a: Amount, b: Amount): Long = a - b
  def leak2: Long = Amount.zero
  def main(args: Array[String]): Unit = println(t(Amount.zero, Amount.zero))
