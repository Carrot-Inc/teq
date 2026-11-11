// A downstream of a/ as tests/modules builds it. Transparent macros whose quotes call inline methods of the same
// object (`'{ M.m(M.k) + 1 }`, `'{ M.d(4) + M.m(M.d(4)) }`); b/'s `run` holds their expansions. scalac 3.8.4 prints
// the .expected, and so do teq's runs. scalac reading teq's pickles of b/ (tests/modules' readtasty-b) fails:
// "assertion failed: position not set for mih.M.d # -1 of class Trees$Select in b/X.scala": the calls the macro's
// result keeps carry no position, as in products_transparent_forwarder_positions. The rows macro cases share fail as
// theirs do (check: the lean std's java.nio.file.Path; scalac-b, scalac-run, scalac-lean-b: no macro implementations
// in teq's class files). Found by reviewer #13 of the workers exports helper (push 8); the same on integration
// b602956824.
package miu8

import mih.M

@main def run(): Unit =
  println(List(M.tvia, M.tdef).mkString(","))
