// A downstream of a/ as tests/modules builds it, reduced from tests/modules/defaultsplan and reviewer #15's probe.
// scalac 3.8.4 prints the .expected, and so does each of teq's whole runs. Over a/'s products teq binds a
// right-associative extension's two parameters swapped: the Scala.js and interpreter runs print `A-R` and `A~R`
// (js-run, interp-run), and the JVM split run `A~R` for the inline `~~:` (jvm-run), where scalac prints `R-A` and
// `R~A`. teq's pickle and scalac's write `--:`'s parameters in dotty's encoded order, (a)(r); the products bundle
// declares them so (`$minus$minus$colon(a, r)`) and b/ passes the operands receiver first, the whole build's order
// (`(r, a)`). `++:`'s default comes out right because it is written over the swapped names and `b` reads no other
// operand. The bundles differ there too (bundle: `$plus$plus$colon(r, a, b = ((a + "/") + r))` against
// `$plus$plus$colon(a, r, b = ((r + "/") + a))`, an own call on `Api$()` against `this`), and the whole and split
// builds' classes of b/ differ (classes, subset). With operands of different types the calls are rejected instead
// (products_right_assoc_extension_operand_types). The same on integration a097de53d9. defaultsplan's bundle row
// shows the parameter order since 3a451f6219 lets its JS products build complete (`a + "/" + r` was withheld before,
// and tests/modules.sh skips the bundle row when that build fails); its whole bundle also lifts `rightAssoc`'s
// operands into vals.
package rdb

import rda.Api
import rda.Api.*

@main def run(): Unit =
  println(("A" ++: "R")())
  println(Api.local())
  println("A" --: "R")
  println("A" ~~: "R")
