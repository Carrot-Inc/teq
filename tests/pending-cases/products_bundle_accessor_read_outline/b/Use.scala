// A downstream of a/ as tests/modules builds
// it. scalac 3.8.4 prints the .expected. The two
// expansions of `add` read C's private `total`
// through the accessors `aroa$C$$inline$total`
// and its setter. teq's Scala.js bundles of the
// whole build and of the products build are to
// be the same (tests/modules' bundle: one file,
// --split and --release alike; accessorcompanion's,
// accessornames', accessorinherited's and
// accessors'). Queued to the representation part.
package arob

@main def run(): Unit =
  val c = new aroa.C()
  println(c.add(2) + c.add(3))
