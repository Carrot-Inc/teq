// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected. dotty
// adds Counter's accessors `amoa$Counter$$inline$bump` and `amoa$Counter$$inline$count`
// after the class's other members (`addAccessorDefs`, `body ++ accDefs`,
// AccessProxies.scala:65-69). teq's Scala.js bundle of the whole build and the bundle built
// over a/'s lean products are to place them alike in the JavaScript class (tests/modules'
// bundle: one file, --split and --release; accessors' and accessorinherited's).
// Queued with the accessors' read (products_bundle_accessor_read_outline).
package amob

@main def run(): Unit =
  val c = new amoa.Counter(1)
  println(c.tick(2) + c.tick(3))
