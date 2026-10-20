enum Opt:
  case A(d: Int)
  case B(d: Int)
  case C(d: Int)

object A:
  def f(x: Opt): Int = x match
    case Opt.A(d) => LocalMacro.run + d
    case Opt.B(d) => d
    case Opt.C(d) => d + 1
  val value: Int = M.next
