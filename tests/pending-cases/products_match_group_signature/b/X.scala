// A downstream of a/ as tests/modules builds it. a/'s plain bodies call `Regex.Match.group`. scalac 3.8.4 prints the
// .expected, and so do teq's JVM runs. scalac reading teq's pickle of a/ (tests/modules' readtasty-a) fails:
// "undefined: mm.group" and "undefined: r.findFirstMatchIn(s).get.group": teq signs `group` with result
// `java.lang.Object` where scalac's is `java.lang.String` (reviewer #14). a/'s lean products withhold both bodies ("the
// std's Match.group, which scala-library has under another shape"), so the Scala.js and interpreter runs over them fail
// (js-run, interp-run). The same before the SELECTtpt port (tip34). Found by reviewer #14 of the workers exports helper
// (push 9).
package gpb

@main def run(): Unit =
  println(gp.G.m("ab"))
  println(gp.G.m3("ab"))
