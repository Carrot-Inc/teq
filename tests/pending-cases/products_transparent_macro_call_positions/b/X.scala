// A downstream of a/ as tests/modules builds it.
// Transparent macros whose quotes call inline
// methods of the same object (`'{ M.m(M.k) + 1 }`,
// `'{ M.d(4) + M.m(M.d(4)) }`); b/'s `run` holds
// their expansions. scalac 3.8.4 prints the
// .expected. teq's pickles of b/ are to be ones
// scalac reads (tests/modules' readtasty-b),
// as in products_transparent_forwarder_positions.
package miu8

import mih.M

@main def run(): Unit =
  println(List(M.tvia, M.tdef).mkString(","))
