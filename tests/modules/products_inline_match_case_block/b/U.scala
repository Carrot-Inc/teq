// scalac 3.8.4 prints run.txt and pickles each inline case body as a BLOCK, including BLOCK(INTconst 0).
// Before the walk's copy-on-write fix, expanding either call changed a stored CaseDef from Block(Nil, n) to n.
// The standalone a/ build kept the block; the whole build lost it, breaking classes, subset and check-lean-whole.
// TreeTypeMap keeps unchanged trees, so the expansion must copy a shared case and its list before replacing its
// body (TreeTypeMap.scala:144-149). A record-node bit is insufficient: stored CaseDefs need not carry that bit.
package ibu

@main def run(): Unit =
  println(ibk.L.first(1))
  println(ibk.L.first(2))
