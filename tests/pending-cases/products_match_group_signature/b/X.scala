// A downstream of a/ as tests/modules builds it. a/'s
// plain bodies call `Regex.Match.group`. scalac 3.8.4
// prints the .expected. teq's pickle of a/ is to be one
// scalac reads (tests/modules' readtasty-a), `group`
// signed with scalac's result, `java.lang.String`. The
// Scala.js and interpreter runs over a/'s lean products
// are to print the .expected too (js-run, interp-run).
package gpb

@main def run(): Unit =
  println(gp.G.m("ab"))
  println(gp.G.m3("ab"))
