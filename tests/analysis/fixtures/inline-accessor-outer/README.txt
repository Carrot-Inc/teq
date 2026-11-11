Recorded scalac 3.8.4 -Ydump-sbt-inc -Yforce-sbt-phases output. A.scala was
compiled first; D.scala was then compiled against A's class directory.
Both dumps match the review's independently recorded used-name and dependency
sets, ignoring ordering.

An inline method in Inner reads and assigns Outer.x. The accessors belong to
Outer, and their names must survive the product boundary into D's expansion.

Compare split-build analysis answers to these dumps with
tests/analysis/check-accessor-names.py. The fixtures record the expected
downstream accessor identities independently of the loader's reconstruction.
