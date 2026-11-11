// Downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, on the JVM and on Scala.js, and so
// does each of teq's runs. a/'s `Use.kinds` holds two expansions of one shape of the transparent `kind`. teq's Scala.js
// bundle of the whole program writes them once, as `kind$o…`, which each site calls (emit/outline.rs). The bundle built
// over a/'s lean products, which reads `kinds` back from a/'s pickle, writes each expansion where it stands
// (tests/modules' bundle: one file, --split and --release alike). The same on integration 2acece915b. master
// efddb38de3's build writes the function in both bundles, which agree there: a TR regression.
// tests/modules/repeatexpand's bundle row shows it since its JS products build completes (the workers exports helper's
// push 10).
package rnb

@main def run(): Unit =
  println(rna.Use.kinds(1))
  println(rna.Use.kinds(5))
