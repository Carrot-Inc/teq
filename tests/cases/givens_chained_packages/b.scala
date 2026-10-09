package a
package b
// The inner clause is a nearer scope than the outer one: its given wins where both provide T,
// and the outer clause's given is still found where the inner one has none.
given bGiven: T with { def n = "b" }
object Run:
  def t = summon[T].n
  def u = summon[U].n
