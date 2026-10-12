package cpb

import cpa.Consts

def cond(): Consts.B.type = Consts.B

@main def run(): Unit =
  println(Consts.K + Consts.T + Consts.I + Consts.Z + Consts.J)
  println(Consts.S + Consts.D + Consts.L + Consts.C)
  println(cond())
  val f: 1.5f = Consts.F
  println(f == 1.5f)
  println(Consts.notConstant)
