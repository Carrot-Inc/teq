package a
package b
package c

// Chained package clauses open every prefix; `package a.b.c` in one clause would open none.
object Chain:
  def run(): Unit =
    println(InB.n)
    println(InA.n)
    println(b.InB.n)
