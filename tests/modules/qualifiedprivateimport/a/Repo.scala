package p.internal

object Repo:
  private[p] enum Side:
    case From
    case Into
  def use(s: Side): String = s.toString
