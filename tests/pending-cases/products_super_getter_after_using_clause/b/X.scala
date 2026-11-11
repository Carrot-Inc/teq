// A downstream of a/ as tests/modules builds it. Traits call through `super` an export's inline method (`mm`) and a
// generic inline method whose defaults are an inline parameter's, a using clause's and a plain one's (`g`). scalac
// 3.8.4 prints the .expected, and so do teq's JVM runs, whole and over the products (over the products integration
// b602956824 throws NoSuchMethodError on `O$.m$default$1`, an inline getter's call, which e23da5da59 expands). Over
// teq's products of a/:
// - scalac reading a/'s pickles (tests/modules' readtasty-a) fails: "method s5$T2$$super$g$default$3 in trait T2 does
//   not take parameters", the super accessor of the getter after the using clause.
// - the Scala.js and interpreter runs fail: typing the products' body of `T2.h` (`super.g[Int](1)()`) reports "type
//   mismatch: found 1, required A @RuntimeChecked (in the body of s5.T2.h)".
// The whole build's classes also differ from the split build's (classes, subset, check-lean-whole). Found by reviewer
// #13 of the workers exports helper (push 8); the same on integration b602956824.
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
