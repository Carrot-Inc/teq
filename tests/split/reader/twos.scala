import fix.rdecl.*

object Twos:
  def run: Int = RdTraces().two + RdBox(2).put("x").length
