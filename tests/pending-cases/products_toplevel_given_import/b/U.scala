// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so does teq's whole build.
// Built over a/'s products, b/ is refused: "no given instance of type T was found", the same for Int (tests/modules'
// build): `import tga.given` finds no top-level given of a product, a given object (`given G2: T with`) or an alias
// (`given x3: Int`). The same on integration a097de53d9. Found by the interim reviewer of the workers exports
// helper's push 10 (its O2).
package tgb

import tga.*
import tga.given

@main def run(): Unit =
  println(summon[T].v + summon[Int])
