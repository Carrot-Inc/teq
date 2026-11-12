// A downstream of a/ as tests/modules builds it. scalac
// 3.8.4 prints the .expected. Built over a/'s products, teq
// is to accept b/ (tests/modules' build): `import tga.given`
// is to find a product's top-level givens, a given object
// (`given G2: T with`) and an alias (`given x3: Int`).
package tgb

import tga.*
import tga.given

@main def run(): Unit =
  println(summon[T].v + summon[Int])
