enum FeeKind:
  case CreditFee
  case CashFee
  case Other(n: Int)

@main def run(): Unit =
  println(Probe.children[FeeKind])
  println(Probe.singletonSymbol[FeeKind.CreditFee.type])
  println(Probe.first[FeeKind])
