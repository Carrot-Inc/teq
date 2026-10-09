package clc

// Each upstream module is built rooted at itself (sourceroots.txt): both keys are x/Use.scala,
// and so are their tokens, which give the two local classes at one offset one name.
@main def run(): Unit = println(s"${cla.A.f}, ${clb.B.f}")
