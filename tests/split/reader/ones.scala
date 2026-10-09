import fix.rdecl.*

object Ones:
  def run: Int = RdTraces().one + RdTraces().wrap(RdTraces().base(5))
