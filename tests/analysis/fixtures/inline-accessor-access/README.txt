Recorded scalac 3.8.4 -Ydump-sbt-inc -Yforce-sbt-phases output. A.scala was
compiled first; D.scala was then compiled against A's class directory.

The protected and qualified-private getter/setter accessors of C must remain
used names when an inline method expands across a product boundary. F and O
exercise final-class and module accessors in the same expansion.

Compare split-build analysis answers to these dumps with
tests/analysis/check-accessor-names.py. The fixtures record the expected
downstream accessor identities independently of the loader's reconstruction.
