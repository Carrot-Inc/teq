// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// Counter's accessors `amoa$Counter$$inline$bump` and `amoa$Counter$$inline$count` come after the class's other
// members in its pickle, as dotty adds them (`addAccessorDefs`, `body ++ accDefs`, AccessProxies.scala:65-69), and so
// in the JavaScript class of the bundle built over a/'s lean products; the whole build's bundle writes `bump`'s
// accessor before `bump` (tests/modules' bundle: one file, --split and --release; accessors' and accessorinherited's),
// the output the same. Master efddb38de3's builds call no accessor and agree: a regression since the whole build calls
// the accessors, queued with the accessors' read (products_bundle_accessor_read_outline).
package amob

@main def run(): Unit =
  val c = new amoa.Counter(1)
  println(c.tick(2) + c.tick(3))
