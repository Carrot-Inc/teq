// A downstream of a/ as tests/modules builds it. Traits call through `super`
// an export's inline method (`mm`) and a generic inline method
// whose defaults are an inline parameter's, a using clause's
// and a plain one's (`g`). scalac 3.8.4 prints the .expected.
// Over the products of a/:
// - a/'s pickles are to be ones scalac reads (tests/modules' readtasty-a),
//   the super accessor of the getter after the using clause too.
// - the Scala.js and interpreter runs
//   are to print the .expected too.
// The whole build's classes are to be the split build's
// (classes, subset, check-lean-whole).
package s5u

import s5.*

trait T3 extends T:
  def f3: Int = super.mm()()
  def h3: String = super.g("q")(using "w")()

class D3 extends T3

@main def run(): Unit =
  val c = C()
  println(List(c.f, c.h).mkString(","))
  val d = D3()
  println(List(d.f3, d.h3).mkString(","))
